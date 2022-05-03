use axum::http::{Request, StatusCode};
use axum::{extract::Query, middleware::Next, response::IntoResponse};
use axum::{Extension, Json};
use serde::{
    de::{self, Deserializer},
    {Deserialize, Serialize},
};
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::Row;
use sqlx::{
    postgres::PgPool,
    types::time::{OffsetDateTime, PrimitiveDateTime},
};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum DataObj {
    Feature { geometry: Geom, properties: Props },
}

#[derive(Serialize, Debug)]
pub enum Props {
    LocProps(LocProps),
    TripProps(TripProps),
}

impl<'de> Deserialize<'de> for Props {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Tag {
            Trip,
        }

        let v = Value::deserialize(deserializer)?;
        match Option::deserialize(&v["type"]).map_err(de::Error::custom)? {
            Some(Tag::Trip) => {
                let inner = TripProps::deserialize(v).map_err(de::Error::custom)?;
                Ok(Props::TripProps(inner))
            }
            None => {
                let inner = LocProps::deserialize(v).map_err(de::Error::custom)?;
                Ok(Props::LocProps(inner))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TripProps {
    #[serde(rename = "device_id")]
    user_id: Option<String>,
    distance: f32,
    duration: f32,
    end: String,
    end_location: Box<DataObj>,
    mode: String,
    start: String,
    start_location: Option<Box<DataObj>>,
    steps: i32,
    stopped_automatically: bool,
    timestamp: String,
    #[serde(rename = "type")]
    ttype: String,
    wifi: String,
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Default)]
pub struct LocProps {
    #[serde(rename = "device_id")]
    user_id: Option<String>,
    activity: Option<String>,
    altitude: Option<i16>,
    battery_level: Option<f32>,
    battery_state: Option<BatteryState>,
    deferred: Option<i32>,
    desired_accuracy: Option<i32>,
    horizontal_accuracy: Option<i32>,
    locations_in_payload: Option<i32>,
    #[serde(default)]
    #[sqlx(default)]
    motion: Vec<Motion>,
    pauses: Option<bool>,
    significant_change: Option<i32>,
    speed: Option<i32>,
    #[sqlx(rename = "time_id")]
    timestamp: String,
    vertical_accuracy: Option<i32>,
    wifi: String,
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "BAT_TYPE", rename_all = "lowercase")]
enum BatteryState {
    Unknown,
    Charging,
    Full,
    Unplugged,
}

#[derive(Serialize, Deserialize, Debug, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
enum Motion {
    Driving,
    Stationary,
    Walking,
    Running,
    Cycling,
}

impl FromStr for Motion {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "driving" => Ok(Motion::Driving),
            "stationary" => Ok(Motion::Stationary),
            "walking" => Ok(Motion::Walking),
            "running" => Ok(Motion::Running),
            "cycling" => Ok(Motion::Cycling),
            m => Err(format!("unknown motion {m}")),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Geom {
    Point { coordinates: [f64; 2] },
}

#[derive(Serialize, Deserialize, Debug)]
struct Locations {
    locations: Vec<DataObj>,
}

#[derive(Serialize)]
pub struct OverlandResponse {
    result: String,
    saved: i32,
}

pub async fn query_points(
    Query(params): Query<HashMap<String, String>>,
    Extension(pool): Extension<PgPool>,
) -> Result<(StatusCode, Json<Vec<DataObj>>), (StatusCode, String)> {
    let offsetdt = params.get("tz").map_or_else(
        || {
            let now = OffsetDateTime::now_utc();
            PrimitiveDateTime::new(now.date(), now.time())
        },
        |tz| PrimitiveDateTime::parse(tz, "%FT%TZ").unwrap(),
    );

    let t_start = offsetdt - Duration::from_secs(3600 * 24);
    let t_end = offsetdt;

    let request = format!(
        r#"SELECT user_id, time_id, altitude, speed, motion, battery, battery_level,
            wifi, coords_x, coords_y FROM points WHERE time_id < TO_TIMESTAMP('{}',
            'YYYY-MM-DD HH24:MI:SS') AND time_id > TO_TIMESTAMP('{}',
            'YYYY-MM-DD HH24:MI:SS');"#,
        t_end.format("%F %T"),
        t_start.format("%F %T"),
    );
    let res: Vec<DataObj> = sqlx::query(&request)
        .map(|row: PgRow| -> sqlx::Result<DataObj> {
            let ts: PrimitiveDateTime = row.try_get("time_id")?;
            let wifi_name: String = row.try_get("wifi")?;
            let motion_string: String = row.try_get("motion")?;
            let motions = motion_string
                .split(",")
                .filter_map(|x| Motion::from_str(x).ok())
                .collect();
            Ok(DataObj::Feature {
                properties: Props::LocProps(LocProps {
                    user_id: row.try_get("user_id")?,
                    timestamp: ts.format("%FT%TZ"),
                    altitude: row.try_get("altitude")?,
                    speed: row.try_get("speed")?,
                    motion: motions,
                    battery_level: row.try_get("battery_level")?,
                    battery_state: row.try_get("battery")?,
                    wifi: wifi_name.trim().to_string(),
                    deferred: None,
                    desired_accuracy: None,
                    horizontal_accuracy: None,
                    vertical_accuracy: None,
                    locations_in_payload: None,
                    activity: None,
                    pauses: None,
                    significant_change: None,
                }),
                geometry: Geom::Point {
                    coordinates: [row.try_get("coords_x")?, row.try_get("coords_y")?],
                },
            })
        })
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|x| {
            if let Ok(c) = x {
                Some(c)
            } else {
                println!("{:?}", x);
                None
            }
        })
        .collect();

    Ok((StatusCode::OK, Json(res)))
}

async fn insert_item(
    geometry: &Geom,
    props: &LocProps,
    pool: &PgPool,
) -> sqlx::Result<sqlx::postgres::PgQueryResult> {
    let point = match geometry {
        Geom::Point { coordinates } => coordinates,
    };
    let offsetdt = PrimitiveDateTime::parse(props.timestamp.clone(), "%FT%TZ").unwrap();
    let user_id = props.user_id.clone().unwrap_or_default();

    sqlx::query!(
        r#"INSERT INTO points (
            user_id, time_id, altitude, speed, motion,
            battery, battery_level, wifi, coords_x, coords_y)
            VALUES ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 )"#,
        user_id,
        offsetdt,
        props.altitude,
        props.speed,
        props
            .motion
            .iter()
            .map(|x| serde_json::to_string(x).unwrap())
            .collect::<Vec<String>>()
            .join(","),
        props.battery_state.unwrap_or(BatteryState::Unknown) as BatteryState,
        props.battery_level,
        props.wifi,
        point[0],
        point[1],
    )
    .execute(pool)
    .await
}

const TOKEN: &'static str = "oPF2pkbvUBqmanC8ft9VbpwJwo9zK3HYkYhyAmps6rJzZHUmDmFGMQyysMnXymgK";

pub async fn auth<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    let auth_header = req.uri().query().unwrap_or("").split("&").any(|x| {
        let split: Vec<String> = x.split("=").map(|x| x.to_string()).collect();
        if (split.len() == 2) && (split[0] == "token") && token_is_valid(&split[1]) {
            true
        } else {
            false
        }
    });

    if auth_header {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn token_is_valid(token: &str) -> bool {
    token == TOKEN
}

pub async fn add_points(
    body: String,
    Query(params): Query<HashMap<String, String>>,
    Extension(pool): Extension<PgPool>,
) -> Result<(StatusCode, Json<OverlandResponse>), (StatusCode, String)> {
    match params.get("token") {
        Some(token) => {
            if token != TOKEN {
                return Err((StatusCode::UNAUTHORIZED, "Unauthorized request".to_string()));
            }
        }
        _ => {}
    }
    let p: Locations = serde_json::from_str(&body).map_err(|e| {
        tracing::debug!("{e}");
        (
            StatusCode::BAD_REQUEST,
            format!("Error parsing data {e}\n Request content: {body}"),
        )
    })?;
    let mut inserted = 0;
    for data_obj in p.locations.iter() {
        match data_obj {
            DataObj::Feature {
                geometry,
                properties,
            } => {
                if let Props::LocProps(props) = properties {
                    match insert_item(&geometry, &props, &pool).await {
                        Ok(_) => inserted += 1,
                        Err(e) => tracing::debug!("error inserting item: {e}"),
                    }
                }
            }
        }
    }
    Ok((
        StatusCode::OK,
        Json(OverlandResponse {
            result: "ok".to_string(),
            saved: inserted,
        }),
    ))
}
