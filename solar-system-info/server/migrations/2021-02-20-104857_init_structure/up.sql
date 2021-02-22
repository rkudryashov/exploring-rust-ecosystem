create table planets (
    id serial primary key,
    name varchar not null unique,
    type varchar(20) not null,
    mean_radius numeric(10,1) not null,
    mass numeric(30) not null
);

create table satellites (
    id serial primary key,
    name varchar not null unique,
    first_spacecraft_landing_date date,
    planet_id integer not null references planets(id)
);
