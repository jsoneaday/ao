import { __, assoc, assocPath, prop, reduce, reduced } from 'ramda'
import { fromPromise, of, Rejected, Resolved } from 'hyper-async'
import AoLoader from '@permaweb/ao-loader'
import { z } from 'zod'

import { saveEvaluationSchema } from '../../dal.js'

/**
 * The result that is produced from this step
 * and added to ctx.
 *
 * This is used to parse the output to ensure the correct shape
 * is always added to context
 */
const ctxSchema = z.object({
  output: z.record(z.any())
}).passthrough()

/**
 * @typedef Env
 * @property {any} db
 */

function addHandler (ctx) {
  return of(ctx.src)
    .map(AoLoader)
    .map((handle) => ({ handle, ...ctx }))
}

function cacheEvaluationWith ({ saveEvaluation, logger }) {
  saveEvaluation = fromPromise(saveEvaluationSchema.implement(saveEvaluation))

  return (evaluation) =>
    of(evaluation)
      .map(logger.tap('Caching evaluation %O'))
      .chain(saveEvaluation)
}

/**
 * @typedef EvaluateArgs
 * @property {string} id - the contract id
 * @property {Record<string, any>} state - the initial state
 * @property {string} from - the initial state sortKey
 * @property {ArrayBuffer} src - the contract wasm as an array buffer
 * @property {Record<string, any>[]} action - an array of interactions to apply
 *
 * @callback Evaluate
 * @param {EvaluateArgs} args
 * @returns {Async<z.infer<typeof ctxSchema>}
 *
 * @param {Env} env
 * @returns {Evaluate}
 */
export function evaluateWith (env) {
  const logger = env.logger.child('evaluate')

  const cacheEvaluation = cacheEvaluationWith({ ...env, logger })

  /**
   * When an error occurs, we short circuit the reduce using
   * ramda's 'reduced()' function, but since our accumulator is an Async,
   * ramda's 'reduce' cannot natively short circuit the reduction.
   *
   * So we do it ourselves by unwrapping the output, and if the value
   * is the 'reduced()' shape, then we immediatley reject, short circuiting the reduction
   *
   * See https://ramdajs.com/docs/#reduced
   * check copied from ramda's internal reduced check impl:
   * https://github.com/ramda/ramda/blob/afe98b03c322fc4d22742869799c9f2796c79744/source/internal/_xReduce.js#L10C11-L10C11
   */
  const maybeReducedError = (With) => (output) => {
    if (output && output['@@transducer/reduced']) {
      return With(output['@@transducer/value'])
    }
    return Resolved(output)
  }
  const maybeResolveError = maybeReducedError(Resolved)
  const maybeRejectError = maybeReducedError(Rejected)

  return (ctx) =>
    of(ctx)
      .chain(addHandler)
      .chain((ctx) =>
        reduce(
          /**
           * See load-actions for incoming shape
           */
          ($output, { action, sortKey, SWGlobal }) =>
            $output
              .chain(maybeRejectError)
              .map(prop('state'))
              .chain((state) =>
                of(state)
                  .chain(
                    fromPromise((state) => ctx.handle(state, action, SWGlobal))
                  )
                  .bichain(
                    /**
                     * Map thrown error to a result.error
                     */
                    (err) => Resolved(assocPath(['result', 'error'], err, {})),
                    Resolved
                  )
                  .chain((output) => {
                    if (output.result && output.result.error) {
                      return Rejected(output)
                    }
                    /**
                     * We default to state to the previous state,
                     * but it will be overwritten by the spread
                     * if output contains state.
                     *
                     * This ensures the new interaction in the chain has state to
                     * operate on, even if the previous interaction only produced
                     * messages and no state change.
                     */
                    return Resolved({ state, ...output })
                  })
              )
              .bimap(
                logger.tap(
                  `Error occurred when applying interaction with sortKey "${sortKey}" to contract "${ctx.id}"`
                ),
                logger.tap(
                  `Applied interaction with sortKey "${sortKey}" to contract "${ctx.id}"`
                )
              )
              /**
               * Create a new interaction to be cached in the local db
               */
              .chain((output) =>
                cacheEvaluation({
                  sortKey,
                  parent: ctx.id,
                  action,
                  output,
                  cachedAt: new Date()
                }).map(() => output)
              )
              .bichain(
                /**
                 * An error was encountered, so stop reduce and return the output
                 */
                (err) => Resolved(reduced(err)),
                /**
                 * Return the output
                 */
                Resolved
              ),
          of({ state: ctx.state, result: ctx.result }),
          ctx.actions
        )
      )
      /**
       * If an error occurred, then it will be wrapped in a reduced,
       * so unwrap it and Resolve, so it can be assigned as output
       * of the evaluation.
       *
       * In other words, this chain should always Resolve
       */
      .chain(maybeResolveError)
      .map(assoc('output', __, ctx))
      .map(ctxSchema.parse)
}