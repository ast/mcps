use crate::error::Error;
use notify_rust::{Notification, Urgency as RustUrgency};

/// Sends desktop notifications via the D-Bus notification protocol.
///
/// Works with any notification daemon that implements the freedesktop.org
/// notification spec, including Mako, dunst, and libnotify.
#[derive(Debug, Clone, Default)]
pub struct NotifyClient;

/// Notification urgency level.
#[derive(Debug, Clone, Copy)]
pub enum Urgency {
    Low,
    Normal,
    Critical,
}

impl From<Urgency> for RustUrgency {
    fn from(u: Urgency) -> Self {
        match u {
            Urgency::Low => RustUrgency::Low,
            Urgency::Normal => RustUrgency::Normal,
            Urgency::Critical => RustUrgency::Critical,
        }
    }
}

impl NotifyClient {
    pub fn new() -> Self {
        Self
    }

    /// Send a desktop notification.
    ///
    /// `summary` is the notification title (required).
    /// `body` is the optional longer description.
    /// `urgency` controls visual priority (default: normal).
    pub fn notify(
        &self,
        summary: &str,
        body: Option<&str>,
        urgency: Option<Urgency>,
    ) -> Result<(), Error> {
        let mut notification = Notification::new();
        notification.summary(summary);

        if let Some(b) = body {
            notification.body(b);
        }

        notification.urgency(urgency.map(Into::into).unwrap_or(RustUrgency::Normal));

        notification.show()?;

        Ok(())
    }
}
