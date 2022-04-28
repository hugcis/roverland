CREATE TYPE MOTION AS ENUM ('driving', 'walking', 'running', 'cycling', 'stationary');
CREATE TYPE BAT_TYPE AS ENUM ('unknown', 'charging', 'full', 'unplugged');
CREATE TABLE points (
	pt_id serial PRIMARY KEY,
    user_id VARCHAR ( 50 ) NOT NULL,
    time_id TIMESTAMP,
    altitude SMALLINT,
    speed INTEGER,
    motion VARCHAR ( 50 ),
    battery BAT_TYPE,
    battery_level REAL,
    wifi CHAR (32),
    coords_x FLOAT,
    coords_y FLOAT
);
