use super::AuthenticatedUser;
use crate::{error::PointercrateError, model::user::User, permissions::Permissions, Result};
use log::{info, trace};
use serde::Deserialize;
use sqlx::PgConnection;

#[derive(Deserialize)]
pub struct Registration {
    pub name: String,
    pub password: String,
}

impl AuthenticatedUser {
    pub async fn register(registration: Registration, connection: &mut PgConnection) -> Result<AuthenticatedUser> {
        info!("Attempting registration of new user under name {}", registration.name);

        Self::validate_password(&registration.password)?;
        User::validate_name(&registration.name)?;

        trace!("Registration request is formally correct");

        match User::by_name(&registration.name, connection).await {
            Ok(_) => Err(PointercrateError::NameTaken),
            Err(PointercrateError::ModelNotFound { .. }) => {
                let hash = bcrypt::hash(&registration.password, bcrypt::DEFAULT_COST).unwrap();

                let id = sqlx::query!(
                    "INSERT INTO members (name, password_hash) VALUES ($1, $2) RETURNING member_id",
                    registration.name,
                    hash
                )
                .fetch_one(connection)
                .await?
                .member_id;

                info!("Newly registered user with name {} has been assigned ID {}", registration.name, id);

                Ok(AuthenticatedUser {
                    user: User {
                        id,
                        name: registration.name,
                        permissions: Permissions::empty(),
                        display_name: None,
                        youtube_channel: None,
                    },
                    password_hash: hash,
                })
            },
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::user::{AuthenticatedUser, Authorization, Registration};

    #[actix_rt::test]
    async fn test_successful_registration() {
        let mut connection = crate::test::test_setup().await;

        let registration = Registration {
            name: "stadust".to_owned(),
            password: "password1234567890".to_owned(),
        };

        let result = AuthenticatedUser::register(registration, &mut connection).await;

        assert!(result.is_ok(), "{:?}", result.err().unwrap());
        assert_eq!(result.unwrap().into_inner().name, "stadust");
    }

    #[actix_rt::test]
    async fn test_successful_basic_auth() {
        let mut connection = crate::test::test_setup().await;

        let result = AuthenticatedUser::basic_auth(
            &Authorization::Basic {
                username: "stadust_existing".to_owned(),
                password: "password1234567890".to_string(),
            },
            &mut connection,
        )
        .await;

        assert!(result.is_ok(), "{:?}", result.err().unwrap());
        assert_eq!(result.unwrap().inner().name, "stadust_existing");
    }
}
