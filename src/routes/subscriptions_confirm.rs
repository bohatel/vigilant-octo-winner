use crate::domain::{SubscriberState, SubscriptionToken};
use crate::routes::errors::ConfirmationError;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = "Confirm a pending subscription"
    skip(_params, db_pool)
)]
pub async fn confirm(
    _params: web::Query<Parameters>,
    db_pool: web::Data<PgPool>,
) -> Result<HttpResponse, ConfirmationError> {
    let subscription_token = SubscriptionToken::parse(_params.subscription_token.clone())
        .map_err(ConfirmationError::ValidationError)?;

    let subscriber_id = get_subscriber_id_from_token(&db_pool, subscription_token.as_ref())
        .await
        .context("Failed to retrieve the subscriber associated with the provided token.")?
        .ok_or(ConfirmationError::UnknownToken)?;

    confirm_subscriber(&db_pool, subscriber_id)
        .await
        .context("Failed to confirm the subscription.")?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Get subscriber id from token"
    skip(db_pool, subscription_token)
)]
pub async fn get_subscriber_id_from_token(
    db_pool: &PgPool,
    subscription_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT subscriber_id FROM subscription_tokens \
        WHERE subscription_token = $1",
        subscription_token,
    )
    .fetch_optional(db_pool)
    .await?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, db_pool))]
pub async fn confirm_subscriber(db_pool: &PgPool, subscriber_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions SET status = $1 WHERE id = $2"#,
        SubscriberState::Active.as_str(),
        subscriber_id,
    )
    .execute(db_pool)
    .await?;
    Ok(())
}
