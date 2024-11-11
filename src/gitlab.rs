//! Handles the communication with gitlab

use core::error::Error;
use core::fmt::{Display, Formatter};
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use tracing::instrument;

/// Defines a gitlab project
#[derive(Debug, Deserialize)]
pub struct Project {
    /// Project id
    pub id: usize,
    /// Project path
    pub path_with_namespace: String,
}

/// cf <https://docs.gitlab.com/ee/api/project_access_tokens.html#create-a-project-access-token>
#[derive(Debug, Deserialize_repr)]
#[repr(u8)]
pub enum AccessLevel {
    Guest = 10,
    Reporter = 20,
    Developer = 30,
    Maintainer = 40,
    Owner = 50,
}

impl Display for AccessLevel {
    #[expect(clippy::min_ident_chars, reason = "Parameter name from std trait")]
    #[expect(clippy::absolute_paths, reason = "Use a specific Result type")]
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Self::Guest => "guest",
                Self::Reporter => "reporter",
                Self::Developer => "developer",
                Self::Maintainer => "maintainer",
                Self::Owner => "owner",
            },
        )
    }
}

/// cf <https://docs.gitlab.com/ee/api/project_access_tokens.html#list-project-access-tokens>
#[derive(Debug, Deserialize)]
pub struct AccessToken {
    /// Scopes
    pub scopes: Vec<String>,
    /// Name
    pub name: String,
    /// Expiration date
    pub expires_at: chrono::NaiveDate,
    /// Active
    pub active: bool,
    /// Revoked
    pub revoked: bool,
    /// Access level
    pub access_level: AccessLevel,
}

/// cf <https://docs.gitlab.com/ee/api/rest/#offset-based-pagination>
pub trait OffsetBasedPagination<T: for<'serde> serde::Deserialize<'serde>> {
    #[instrument(err, skip_all, target = "gitlab")]
    async fn get_all(
        http_client: &reqwest::Client,
        url: String,
        gitlab_token: &str,
    ) -> Result<Vec<T>, Box<dyn Error + Send + Sync>> {
        let mut result: Vec<T> = Vec::new();
        let mut next_url: Option<String> = Some(url);

        while let Some(value) = next_url {
            let resp = http_client
                .get(value)
                .header("PRIVATE-TOKEN", gitlab_token)
                .send()
                .await?
                .error_for_status()?;

            next_url = resp
                .headers()
                .get("link")
                .and_then(|header_value| header_value.to_str().ok())
                .and_then(|header_value_str| {
                    parse_link_header::parse_with_rel(header_value_str).ok()
                })
                .and_then(|links| links.get("next").map(|link| link.raw_uri.clone()));

            let mut items: Vec<T> = resp.json().await?;
            result.append(&mut items);
        }

        Ok(result)
    }
}

#[expect(clippy::missing_trait_methods, reason = "we don't need it")]
impl OffsetBasedPagination<Self> for Project {}
#[expect(clippy::missing_trait_methods, reason = "we don't need it")]
impl OffsetBasedPagination<Self> for AccessToken {}
