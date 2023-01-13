  ![Logo](assets/logo.png)

## Description

Roverland is a backend server and web app created for the Overland-iOS app. This
is a personal project I started to keep track of my location in a
privacy-preserving way. I self-host a Roverland server which has been tracking
my location for several months as an experiment.

The web app is still quite simple and lacks many features, but it's enough to
roughly explore my past locations.

I have opened registration to a few friends who also track their location with
it, but I'm unable to accept any outside registrations right now.

## Build

You need a running Postgres database (by default on localhost). 

``` sh
psql template1 -c 'CREATE USER overland WITH PASSWORD ${YOUR_PASSWORD};'
psql template1 -c 'CREATE DATABASE overland_db WITH OWNER overland;'
```

Make sure all the parameters (including password and database location) are
right in the `.env` file. The database is now ready.

Then, building is as simple as 
``` sh
cargo build 
```

