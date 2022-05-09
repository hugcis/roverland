--create types
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'motion') THEN
        CREATE TYPE MOTION AS ENUM ('driving', 'walking', 'running', 'cycling', 'stationary');
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'bat_type') THEN
        CREATE TYPE BAT_TYPE AS ENUM ('unknown', 'charging', 'full', 'unplugged');
    END IF;
END$$;
CREATE TABLE IF NOT EXISTS points (
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
CREATE TABLE IF NOT EXISTS users (
  id SERIAL PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  password TEXT NOT NULL
);
