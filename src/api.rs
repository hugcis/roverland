/// The API module contains all the API method implementations for the REST
/// server.
use super::auth::CurrentUser;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Extension, Json};
use serde::{
    de::{self, Deserializer},
    {Deserialize, Serialize},
};
use serde_json::Value;
use sqlx::Row;
use sqlx::{
    postgres::{PgPool, PgRow},
    types::time::{Date, PrimitiveDateTime},
};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use time::macros::format_description;

/// The DataObj enum represents the objects received by a client containing the
/// locations of the user.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum DataObj {
    /// The Feature corresponds to a GeoJSON feature object. It contains a
    /// `geometry` object with the object type and coordinates and the
    /// `properties` containing options for the object.
    Feature {
        /// The `geometry` object.
        geometry: Geom,
        /// The `properties` object.
        properties: Props,
    },
}

/// The GeoJSON properties can be of two type: single location properties
/// `LocProps` or trip-properties `TripProps`. These correspond to the two types
/// of object that the Overland iOS app client can send.
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum Props {
    /// A single location property object.
    LocProps(LocProps),
    /// A trip property object.
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

/// A trip properties object, containing the starting and ending point of the
/// trip and some additional properties.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename = "trip")]
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
    wifi: String,
}

/// A location properties object.
#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Default)]
#[serde(rename = "position")]
pub struct LocProps {
    #[serde(rename = "device_id")]
    user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    altitude: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    battery_level: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    battery_state: Option<BatteryState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    horizontal_accuracy: Option<i32>,
    #[serde(default)]
    #[sqlx(default)]
    motion: Vec<Motion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speed: Option<i32>,
    #[sqlx(rename = "time_id")]
    timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    vertical_accuracy: Option<i32>,
    wifi: String,
}

/// The `BatteryState` returned from the Overland app can take a few values.
#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "BAT_TYPE", rename_all = "lowercase")]
pub enum BatteryState {
    #[default]
    /// Battery state unknown
    Unknown,
    /// Battery is charging
    Charging,
    /// Battery is full but plugged in
    Full,
    /// Battery is unplugged
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

/// A geometry object that contains geometric properties of a GeoJSON object.
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Geom {
    /// The geometry is a point.
    Point {
        /// Latitude and longitude of the point.
        coordinates: [f64; 2],
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct Locations {
    locations: Vec<DataObj>,
}

/// This is the standard response to an `add_points` POST request.
#[derive(Serialize)]
pub struct OverlandResponse {
    result: String,
    saved: i32,
}

/// A time period interval for the `GeoQuery::Interval` object.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TimePeriod {
    /// One day time period.
    Day,
    /// One week time period.
    Week,
    /// One month time period.
    Month,
}

impl Default for TimePeriod {
    fn default() -> Self {
        Self::Day
    }
}

/// An enum representing the required result type for the returned data from a
/// query.
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum ResultType {
    /// This is the internal Json type.
    #[default]
    Json,
    /// Standard GeoJSON type.
    GeoJSON,
}

/// An enum representing the response type that corresponds to the queried
/// `ResultType`.
pub enum QueryPointResponse {
    /// The GeoJSON variant contains an axum Json representation of a `Vec` of
    /// `DataObj`.
    GeoJSON(Json<Vec<DataObj>>),
    /// The Json variant is a Json representaion of a `PositionCollection`.
    Json(Json<PositionCollection>),
}

/// A GeoQuey is a query parameter group that can either be an interval with a
/// start date and end date or a single date (`TimePeriod::Day`,
/// `TimePeriod::Month`, etc.) request.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
#[serde(untagged)]
pub enum GeoQuery {
    /// An Interval geo query type.
    Interval {
        /// The start date of the interval. The expected format is
        /// `[year]-[month]-[day]T[hour]:[minute]:[second] :00`
        start: String,
        /// The end date of the interval.
        end: String,
        /// Results can be required to be in any of the `ResultType`.
        #[serde(default)]
        result_type: ResultType,
    },
    /// An single day geo query type.
    Date {
        /// The starting date of the required data.
        date: String,
        /// The duration of the required data
        #[serde(default)]
        duration: TimePeriod,
    },
}

fn geoquery_to_primitive_datetime(
    geo_query: GeoQuery,
) -> (PrimitiveDateTime, PrimitiveDateTime, ResultType) {
    let formatter = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second] 00:00");
    match geo_query {
        GeoQuery::Date { date, duration } => {
            let offsetdt = Date::parse(&date, formatter).unwrap();
            let t_start = match duration {
                TimePeriod::Day => offsetdt.midnight(),
                TimePeriod::Week => offsetdt.midnight() - Duration::from_secs(3600 * 24 * 7),
                TimePeriod::Month => offsetdt.midnight() - Duration::from_secs(3600 * 24 * 30),
            };
            let t_end = offsetdt.next_day().unwrap().midnight();
            (t_start, t_end, ResultType::Json)
        }
        GeoQuery::Interval {
            start,
            end,
            result_type,
        } => {
            let start_dt = Date::parse(&start, formatter).unwrap().midnight();
            let end_dt = Date::parse(&end, formatter).unwrap().midnight();
            (start_dt, end_dt, result_type)
        }
    }
}

type PositionTuple = (f32, f32, i16, f32, u8, String, u16, i32);

/// The position collection format. Used to encode a long list of positions more
/// efficiently than with a GeoJSON.
#[derive(Serialize)]
pub struct PositionCollection {
    wifis: Vec<String>,
    states: Vec<BatteryState>,
    devices: HashMap<String, Vec<PositionTuple>>,
}

fn dataobj_vec_to_internal(dobj_vec: Vec<DataObj>) -> PositionCollection {
    let mut map = HashMap::new();
    let mut wifi_map = HashMap::new();
    let mut wifi_array = vec![];
    let mut bstate_map = HashMap::new();
    let mut bstate_array = vec![];

    for obj in dobj_vec.iter() {
        match obj {
            DataObj::Feature {
                geometry,
                properties,
            } => {
                if let Props::LocProps(props) = properties {
                    let id_str = props.user_id.as_ref().unwrap();
                    if !map.contains_key(id_str) {
                        map.insert(id_str.clone(), vec![]);
                    }
                    let id_vec = map.get_mut(id_str).unwrap();
                    let battery_state = props.battery_state.unwrap_or_default();
                    let bstate_index = if let Some(bstate_index) = bstate_map.get(&battery_state) {
                        *bstate_index
                    } else {
                        let bstate_index = bstate_map.len();
                        bstate_map.insert(battery_state, bstate_index);
                        bstate_array.push(battery_state);
                        bstate_index
                    };
                    let wifi = props.wifi.clone();
                    let wifi_index = if let Some(wifi_index) = wifi_map.get(&wifi) {
                        *wifi_index
                    } else {
                        let wifi_index = wifi_map.len();
                        wifi_map.insert(wifi.clone(), wifi_index);
                        wifi_array.push(wifi);
                        wifi_index
                    };

                    match geometry {
                        Geom::Point { coordinates } => {
                            id_vec.push((
                                coordinates[0] as f32,
                                coordinates[1] as f32,
                                props.altitude.unwrap_or(0),
                                props.battery_level.unwrap_or(1.),
                                bstate_index.try_into().unwrap_or_else(|_| {
                                    tracing::error!("Index for battery state too high");
                                    0
                                }),
                                props.timestamp.clone(),
                                wifi_index.try_into().unwrap_or_else(|_| {
                                    tracing::error!("Index for wifi too high");
                                    0
                                }),
                                props.speed.unwrap_or(0),
                            ));
                        }
                    }
                }
            }
        }
    }
    PositionCollection {
        wifis: wifi_array,
        states: bstate_array,
        devices: map,
    }
}

impl IntoResponse for QueryPointResponse {
    fn into_response(self) -> Response {
        match self {
            Self::GeoJSON(response) => response.into_response(),
            Self::Json(response) => response.into_response(),
        }
    }
}

fn filter_results(current_user: CurrentUser, first: bool) -> String {
    if current_user.is_admin {
        "".to_string()
    } else if first {
        format!("WHERE user_identifier={}", current_user.user_id)
    } else {
        format!("AND user_identifier={}", current_user.user_id)
    }
}

/// API method to get the dates with existing data for a specific user.
pub async fn available(
    Extension(pool): Extension<PgPool>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<(StatusCode, Json<Vec<String>>), (StatusCode, String)> {
    let formatter = format_description!("[year]-[month]-[day]");
    let res: Vec<Date> = sqlx::query(&format!(
        r#"SELECT DISTINCT DATE(time_id) AS single_day FROM points {};"#,
        filter_results(current_user, true)
    ))
    .map(|row: PgRow| -> sqlx::Result<sqlx::types::time::Date> {
        let tget = row.try_get("single_day");
        tget
    })
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .collect::<sqlx::Result<Vec<Date>>>()
    .unwrap();
    let formatted_dates = res
        .iter()
        .map(|dt| (*dt).format(&formatter))
        .collect::<Result<Vec<String>, time::error::Format>>()
        .unwrap();
    Ok((StatusCode::OK, Json(formatted_dates)))
}

/// API method to query positions from the database for a specific user.
pub async fn query_points(
    Query(geo_query): Query<GeoQuery>,
    Extension(pool): Extension<PgPool>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<(StatusCode, QueryPointResponse), (StatusCode, String)> {
    let (t_start, t_end, result_type) = geoquery_to_primitive_datetime(geo_query);
    let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let request = format!(
        r#"SELECT user_id, time_id, altitude, speed, motion, battery, battery_level,
            wifi, coords_x, coords_y FROM points WHERE time_id BETWEEN TO_TIMESTAMP('{}',
            'YYYY-MM-DD HH24:MI:SS') AND TO_TIMESTAMP('{}',
            'YYYY-MM-DD HH24:MI:SS') {};"#,
        t_start.format(&format).unwrap(),
        t_end.format(&format).unwrap(),
        filter_results(current_user, false)
    );
    let format_date = format_description!("[year]-[month]-[day]");
    let res: Vec<DataObj> = sqlx::query(&request)
        .map(|row: PgRow| -> sqlx::Result<DataObj> {
            let ts: PrimitiveDateTime = row.try_get("time_id")?;
            let wifi_name: String = row.try_get("wifi")?;
            let motion_string: String = row.try_get("motion")?;
            let motions = motion_string
                .split(',')
                .filter_map(|x| Motion::from_str(x).ok())
                .collect();
            Ok(DataObj::Feature {
                properties: Props::LocProps(LocProps {
                    user_id: row.try_get("user_id")?,
                    timestamp: format!(
                        "{}T{:02}:{:02}:{:02}Z",
                        ts.format(&format_date).unwrap(),
                        ts.hour(),
                        ts.minute(),
                        ts.second()
                    ),
                    altitude: row.try_get("altitude")?,
                    speed: row.try_get("speed")?,
                    motion: motions,
                    battery_level: row.try_get("battery_level")?,
                    battery_state: row.try_get("battery")?,
                    wifi: wifi_name.trim().to_string(),
                    horizontal_accuracy: None,
                    vertical_accuracy: None,
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
        .filter_map(|x| if let Ok(c) = x { Some(c) } else { None })
        .collect();
    match result_type {
        ResultType::GeoJSON => Ok((StatusCode::OK, QueryPointResponse::GeoJSON(Json(res)))),
        ResultType::Json => Ok((
            StatusCode::OK,
            QueryPointResponse::Json(Json(dataobj_vec_to_internal(res))),
        )),
    }
}

async fn insert_item(
    geometry: &Geom,
    props: &LocProps,
    pool: &PgPool,
    current_user: &CurrentUser,
) -> sqlx::Result<sqlx::postgres::PgQueryResult> {
    let point = match geometry {
        Geom::Point { coordinates } => coordinates,
    };
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");
    let offsetdt = PrimitiveDateTime::parse(&props.timestamp, format).unwrap();
    let user_id = props.user_id.clone().unwrap_or_default();

    sqlx::query!(
        r#"INSERT INTO points (
            user_id, time_id, altitude, speed, motion,
            battery, battery_level, wifi, coords_x, coords_y, user_identifier)
            VALUES ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11 )"#,
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
        current_user.user_id
    )
    .execute(pool)
    .await
}

/// API method to insert new points into the DB. This is called when POST-ing
/// new data from the app.
pub async fn add_points(
    body: String,
    Extension(pool): Extension<PgPool>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<(StatusCode, Json<OverlandResponse>), (StatusCode, String)> {
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
                    match insert_item(geometry, props, &pool, &current_user).await {
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
