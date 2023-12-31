use crate::domain::{
    NewSubscriber, SubscriberEmail, SubscriberName, SubscriberState, SubscriptionToken,
};
use crate::email_client::EmailClient;
use crate::routes::errors::{StoreTokenError, SubscribeError};
use crate::startup::{ApplicationBaseUrl, TEMPLATES};
use actix_web::{web, HttpResponse};
use anyhow::Context;
use chrono::Utc;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

struct SubscriberSatus {
    id: Uuid,
    status: SubscriberState,
}

#[tracing::instrument(
    name = "Adding new subscriber",
    skip(form, db_pool, email_client, base_url),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    db_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, SubscribeError> {
    let new_subscriber = form.0.try_into().map_err(SubscribeError::ValidationError)?;

    let mut transaction = db_pool
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;
    let mut temporary_id = insert_subscriber(&mut transaction, &new_subscriber).await;
    if temporary_id.is_err() {
        temporary_id =
            match get_subscriber_id_from_email(&db_pool, new_subscriber.email.as_ref()).await {
                Some(r) => {
                    match r.status {
                        SubscriberState::Active => return Ok(HttpResponse::Ok().finish()),
                        _other => {
                            let _ = transaction.rollback().await;
                            transaction = db_pool
                                .begin()
                                .await
                                .context("Failed to acquire a Postgres connection from the pool")?;
                            set_subscriber_pending(&mut transaction, r.id)
                                .await
                                .context("Failed to update subscriber status")?;
                        }
                    }
                    Ok(r.id)
                }
                None => temporary_id,
            }
    }
    let subscriber_id = temporary_id.context("Failed to insert new subscriber in the database.")?;

    let subscription_token = SubscriptionToken::generate();
    save_token(&mut transaction, subscriber_id, subscription_token.as_ref())
        .await
        .context("Failed to store the confirmation token for a new subscriber.")?;
    transaction
        .commit()
        .await
        .context("Failed to commit SQL transaction to store a new subscriber.")?;
    send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        subscription_token.as_ref(),
    )
    .await
    .context("Failed to send a confirmation email.")?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Send confirmation email to a new subscriber",
    skip(email_client, new_subscriber, subscription_token)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), SubscribeError> {
    let confirmation_link =
        format!("{base_url}/subscriptions/confirm?subscription_token={subscription_token}");
    let mut context = tera::Context::new();
    context.insert("confirmation_link", &confirmation_link);

    let plain_body = TEMPLATES
        .render("email/welcome.txt", &context)
        .context("Failed to parse plain text template")?;
    let html_body = TEMPLATES
        .render("email/welcome.html", &context)
        .context("Failed to parse plain text template")?;

    email_client
        .send_email(new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await
        .context("Failed to send email.")?;

    Ok(())
}

#[tracing::instrument(
    name = "Saving subscriber details in the database",
    skip(new_subscriber, transaction)
)]
pub async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let query = sqlx::query!(
        r#"
    INSERT INTO subscriptions (id, email, name, subscribed_at, status)
    VALUES ($1, $2, $3, $4, $5)
        "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
        SubscriberState::Pending.as_str()
    );
    transaction.execute(query).await?;
    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Updating subscriber status to pending_confirmation",
    skip(subscriber_id, transaction)
)]
pub async fn set_subscriber_pending(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
) -> Result<(), sqlx::Error> {
    let query = sqlx::query!(
        r#"
    UPDATE subscriptions set status = $1
    WHERE id = $2
        "#,
        SubscriberState::Pending.as_str(),
        subscriber_id,
    );
    transaction.execute(query).await?;
    Ok(())
}

#[tracing::instrument(
    name = "Saving subscribtion token in the database",
    skip(transaction, subscriber_id, subscription_token)
)]
pub async fn save_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), StoreTokenError> {
    let query = sqlx::query!(
        r#"INSERT INTO subscription_tokens (subscription_token, subscriber_id)
        VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    );
    transaction.execute(query).await.map_err(StoreTokenError)?;
    Ok(())
}

// We should only call this is insert fails and don't need to bouble up errors
#[tracing::instrument(name = "Find subscriber by email", skip(subscriber_email, db_pool))]
async fn get_subscriber_id_from_email(
    db_pool: &PgPool,
    subscriber_email: &str,
) -> Option<SubscriberSatus> {
    match sqlx::query!(
        "SELECT id, status FROM subscriptions WHERE email = $1",
        subscriber_email,
    )
    .fetch_optional(db_pool)
    .await
    {
        Ok(result) => result.map(|r| {
            let status = r.status.try_into().unwrap_or(SubscriberState::Disabled);
            SubscriberSatus { id: r.id, status }
        }),
        Err(e) => {
            tracing::error!("Failed to query subscriber by email: {:?}", e);
            None
        }
    }
}
