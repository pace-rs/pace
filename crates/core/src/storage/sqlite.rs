use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

use crate::{
    domain::activity::{Activity, ActivityId, ActivityLog},
    error::PaceResult,
    storage::ActivityStorage,
};

struct SqliteActivityStorage {
    conn: Connection,
}

impl SqliteActivityStorage {
    fn new(db_path: &str) -> PaceResult<Self> {
        let conn = Connection::open(db_path)?;
        // Ensure the table for activities exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS activities (
                id              INTEGER PRIMARY KEY,
                description     TEXT NOT NULL
                -- Add other fields as necessary
            )",
            [],
        )?; // TODO!
        Ok(Self { conn })
    }
}

impl ActivityStorage for SqliteActivityStorage {
    fn load_activities(&self) -> PaceResult<ActivityLog> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, description FROM activities")?; // TODO!
        let activity_iter = stmt.query_map([], |row| {
            Ok(Activity::builder()
                .id(ActivityId::from(row.get(0)?)?)
                .description(row.get(1)?)
                .category(row.get(3)?)
                .end_date(row.get(4)?)
                .end_time(row.get(5)?)
                .start_date(row.get(6)?)
                .start_time(row.get(7)?)
                .kind(row.get(8)?)
                .subcategory(row.get(9)?)
                .tags(row.get(10)?)
                .tasks(row.get(11)?)
                .pomodoro_cycle(row.get(12)?)
                .intermission_periods(row.get(13)?)
                .build()?)
        })?;

        let mut activities = Vec::new();
        for activity in activity_iter {
            activities.push(activity?);
        }
        Ok(activities)
    }

    fn save_activity(&mut self, activity: &Activity) -> PaceResult<()> {
        self.conn.execute(
            "INSERT INTO activities (id, description) VALUES (?1, ?2)",
            params![activity.id(), activity.description()],
        )?; // TODO!
        Ok(())
    }
}
