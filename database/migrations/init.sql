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
  coords_y FLOAT,
  user_identifier INT NOT NULL,
  CONSTRAINT user_cst FOREIGN KEY(user_identifier) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS users (
  id SERIAL PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  password TEXT NOT NULL,
  is_admin BOOLEAN
);

CREATE TABLE IF NOT EXISTS register_tokens (
  id SERIAL PRIMARY KEY,
  register_token VARCHAR (64) NOT NULL UNIQUE,
  used BOOLEAN
);

CREATE TABLE IF NOT EXISTS input_tokens (
  id SERIAL PRIMARY KEY,
  input_token VARCHAR (64) NOT NULL UNIQUE,
  valid BOOLEAN,
  user_id INT,
  CONSTRAINT user_cst FOREIGN KEY(user_id) REFERENCES users(id)
);
