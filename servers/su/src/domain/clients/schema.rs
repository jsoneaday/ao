use diesel::prelude::*;

table! {
    processes (row_id) {
        row_id -> Integer,
        process_id -> Varchar,
        process_data -> Jsonb,
    }
}

table! {
    messages (row_id) {
        row_id -> Integer,
        process_id -> Varchar,
        message_id -> Varchar,
        sort_key -> Varchar,
        message_data -> Jsonb,
    }
}

table! {
    schedulers (row_id) {
        row_id -> Integer,
        url -> Varchar,
    }
}

joinable!(processes -> schedulers (scheduler_row_id)); // establishes the foreign key relationship
allow_tables_to_appear_in_same_query!(processes, schedulers);