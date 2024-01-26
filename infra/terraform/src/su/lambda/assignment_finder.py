import json
import os
import boto3
import urllib.request
from botocore.exceptions import ClientError



# Initialize a DynamoDB client
dynamodb_client = boto3.resource('dynamodb')

application_port = int(os.environ['APPLICATION_PORT'])
su_unit_count = int(os.environ['SU_UNIT_COUNT'])
ddb_su_assignments_table_name = os.environ['DDB_SU_ASSIGNMENTS_TABLE_NAME']

def get_secret(secret_name, region_name):
    # Create a Secrets Manager client
    session = boto3.session.Session()
    client = session.client(
        service_name='secretsmanager',
        region_name=region_name
    )

    try:
        get_secret_value_response = client.get_secret_value(
            SecretId=secret_name
        )
    except ClientError as e:
        if e.response['Error']['Code'] == 'ResourceNotFoundException':
            print("The requested secret " + secret_name + " was not found")
        elif e.response['Error']['Code'] == 'InvalidRequestException':
            print("The request was invalid due to:", e)
        elif e.response['Error']['Code'] == 'InvalidParameterException':
            print("The request had invalid params:", e)
        elif e.response['Error']['Code'] == 'DecryptionFailure':
            # Secrets Manager can't decrypt the protected secret text using the provided KMS key.
            # Deal with the exception here, and/or rethrow at your discretion.
            raise
        elif e.response['Error']['Code'] == 'InternalServiceError':
            # An error occurred on the server side.
            # Deal with the exception here, and/or rethrow at your discretion.
            raise
        else:
            # Handle any other exceptions that may be raised.
            raise
    else:
        # Decrypts secret using the associated KMS CMK.
        # Depending on whether the secret is a string or binary, one of these fields will be populated.
        if 'SecretString' in get_secret_value_response:
            text_secret_data = get_secret_value_response['SecretString']
            return text_secret_data
        else:
            binary_secret_data = get_secret_value_response['SecretBinary']
            return binary_secret_data

slack_oauth_token = get_secret('forward-research-slackbot-oauth-token', 'us-west-1')

def send_slack_message(custom_message, **kwargs):
  headers = {
    'Accept': 'application/json',
    'Content-Type': 'application/json; charset=utf-8',
    'Authorization': f'Bearer {slack_oauth_token}',
  }
  alert_state = kwargs.get('alert_state', None)
  payload = {
    'channel': 'C06FC5HDFT5',
    'username': 'Testnet-AO-DevOps',
    'icon_emoji': ':diamond_shape_with_a_dot_inside:',
    'unfurl_links': False,
    'unfurl_media': False,
  }

  if not alert_state:
    payload['blocks'] = [
      {
        'type': 'section',
        'text': {
          'type': 'mrkdwn',
          'text': custom_message,
        },
      },
    ]
  elif alert_state == 'ok':
    payload['attachments'] = [
        {
            'color': '#36a64f',
            'text': custom_message,
        }
    ]
  elif alert_state == 'warn':
    payload['attachments'] = [
        {
            'color': '#FFA500',
            'text': custom_message,
        }
    ]
  else:
    payload['attachments'] = [
        {
            'color': '#ff0000',
            'text': custom_message,
        }
    ]

  data=json.dumps(payload).encode("utf-8")
  req = urllib.request.Request("https://slack.com/api/chat.postMessage", data=data, headers=headers)

  try:
    with urllib.request.urlopen(req) as response:
      print(f"Slack response status: {response.getcode()}") # getcode() gets the HTTP status code
      print(f"Slack response body: {response.read().decode()}") # read the response and decode it
  except Exception as e:
      print(f"Error sending Slack message: {e}")


def lambda_handler(event, context):
    eventstr = json.dumps(event, indent=2)
    print("Received event: " + eventstr)
    table = dynamodb_client.Table(ddb_su_assignments_table_name)
    new_public_ip = event['public_ip']

    if not new_public_ip or new_public_ip == "":
        print("No public IP address found, skipping")
        send_slack_message("Assignment finder lambda event error: " + eventstr, alert_state="warn")
        return "ERROR BAD EVENT"

    # Loop over the assignment numbers
    for assignment_number in range(1, su_unit_count + 1):

        # Get the item by AssignmentNumber
        ddb_response = table.get_item(
            Key={
                'AssignmentNumber': assignment_number
            }
        )

        item = ddb_response.get('Item')

        if item and 'public_ip' in item:
          print(f"ddb respone for su{assignment_number}, public_ip: {item['public_ip']}")

        # If the item is missing or public_ip is empty, update it
        if not item or ('public_ip' not in item or not item['public_ip']) or item['public_ip'] == "null" or item['public_ip'] == None:
            # Update the item with the new public IP address
            update_response = table.update_item(
                Key={
                    'AssignmentNumber': assignment_number
                },
                UpdateExpression='SET public_ip = :val',
                ExpressionAttributeValues={
                    ':val': new_public_ip
                },
                ReturnValues='UPDATED_NEW'
            )

            print(f"Updated item: {json.dumps(update_response, indent=2)}")
            send_slack_message(f"Assignment finder: {new_public_ip} was assigned to su{assignment_number}.ao-testnet.xyz:{application_port}", alert_state="ok")
            # Return the AssignmentNumber of the updated item
            return assignment_number

    send_slack_message(f"Assignment finder: {new_public_ip} was unable to find available assignment", alert_state="error")
    return "ERROR NO ASSIGNMENT FOUND"
