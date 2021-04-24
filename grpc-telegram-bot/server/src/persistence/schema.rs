table! {
    planets (id) {
        id -> Int4,
        name -> Varchar,
        #[sql_name = "type"]
        type_ -> Varchar,
        mean_radius -> Numeric,
        mass -> Numeric,
    }
}

table! {
    satellites (id) {
        id -> Int4,
        name -> Varchar,
        first_spacecraft_landing_date -> Nullable<Date>,
        planet_id -> Int4,
    }
}

joinable!(satellites -> planets (planet_id));

allow_tables_to_appear_in_same_query!(planets, satellites,);
