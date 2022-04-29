use axum::http::StatusCode;
use axum::{extract::Query, routing::post, Router};
use std::env;

use axum::{Extension, Json};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::collections::HashMap;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum DataObj {
    Feature { geometry: Geom, properties: Props },
}

#[derive(Serialize, Debug)]
enum Props {
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
struct TripProps {
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

#[derive(Serialize, Deserialize, Debug)]
struct LocProps {
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
    motion: Vec<Motion>,
    pauses: Option<bool>,
    significant_change: Option<i32>,
    speed: Option<i32>,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Motion {
    Driving,
    Stationary,
    Walking,
    Running,
    Cycling,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Geom {
    Point { coordinates: [f64; 2] },
}

#[derive(Serialize, Deserialize, Debug)]
struct Locations {
    locations: Vec<DataObj>,
}

#[derive(Serialize)]
struct OverlandResponse {
    result: String,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL").unwrap())
        .await?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
        .route("/", post(add_points))
        .layer(Extension(pool))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 18032));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn add_points(
    body: String,
    Query(_params): Query<HashMap<String, String>>,
    Extension(pool): Extension<PgPool>,
) -> Result<(StatusCode, Json<OverlandResponse>), (StatusCode, String)> {
    let p: Locations = serde_json::from_str(&body).map_err(|e| {
        tracing::debug!("{e}");
        println!("{e}");
        println!("{body}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error parsing data {e}\n Content: {body}"),
        )
    })?;
    for data_obj in p.locations.iter() {
        match data_obj {
            DataObj::Feature {
                geometry,
                properties,
            } => {
                if let Props::LocProps(props) = properties {
                    let point = match geometry {
                        Geom::Point { coordinates } => coordinates,
                    };
                    let offsetdt = sqlx::types::time::PrimitiveDateTime::parse(
                        props.timestamp.clone(),
                        "%FT%TZ",
                    )
                    .unwrap();
                    let user_id = props.user_id.clone().unwrap_or_default();
                    sqlx::query!(
                        r#"INSERT INTO points ( user_id, time_id, altitude, speed, motion, battery, battery_level, wifi, coords_x, coords_y)
VALUES ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10 ) RETURNING pt_id"#,
                        user_id,
                        offsetdt,
                        props.altitude,
                        props.speed,
                        props.motion.iter().map(|x| serde_json::to_string(x).unwrap()).collect::<Vec<String>>().join(","),
                        props.battery_state.unwrap_or(BatteryState::Unknown) as BatteryState,
                        props.battery_level,
                        props.wifi,
                        point[0],
                        point[1],
                    )
                    .fetch_one(&pool).await.unwrap();
                }
            }
        }
    }
    Ok((
        StatusCode::OK,
        Json(OverlandResponse {
            result: "ok".to_string(),
        }),
    ))
}
