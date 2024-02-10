//! Event handling

#[derive(Clone, Debug, PartialEq, Eq)]
enum PaceEventType {
    // Task-related events
    // Cover the lifecycle of tasks from beginning, ending, pausing, and resuming.
    TaskCreated {
        task_id: u32,
        description: String,
    },
    TaskStarted {
        task_id: u32,
    },
    TaskPaused {
        task_id: u32,
    },
    TaskResumed {
        task_id: u32,
    },
    ResumeTask {
        task_id: u32,
    },

    // Intermission-related events
    // Represent the start and end of an intermission, which are purposeful breaks or pauses.
    BeginIntermission {
        intermission_id: u32,
    },
    EndIntermission {
        intermission_id: u32,
    },

    // Pomodoro-related events
    // Reflect the structured work-break cycles of the Pomodoro technique, differentiating between work periods and break periods.
    BeginPomodoroWork {
        pomodoro_id: u32,
    },
    EndPomodoroWork {
        pomodoro_id: u32,
    },
    BeginPomodoroBreak {
        pomodoro_id: u32,
    },
    EndPomodoroBreak {
        pomodoro_id: u32,
    },

    // General activity events
    // A more generalized way to handle any activity, whether it's a task, an intermission, or part of a Pomodoro session. This approach provides flexibility for future extensions.
    BeginActivity {
        activity_id: u32,
        activity_type: ActivityType,
    },
    EndActivity {
        activity_id: u32,
        activity_type: ActivityType,
    },

    // Additional system events
    // Include events for user actions (like login and logout) and system errors, ensuring comprehensive event handling that can support a wide range of functionalities and system states.
    UserLogin {
        user_id: u32,
    },
    UserLogout {
        user_id: u32,
    },
    SystemError {
        error_code: String,
        message: String,
    },

    // Testing
    #[cfg(test)]
    Test,
}
