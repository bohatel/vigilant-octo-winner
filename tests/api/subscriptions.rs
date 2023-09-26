use crate::helpers::{setup_with_mock, spawn_app};
use zero2prod::domain::SubscriberState;

#[tokio::test]
async fn subscribe_returns_200_for_valid_data() {
    let app = setup_with_mock().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_returns_400_when_data_is_missing_or_invalid() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return 400 Bad Request when the payload was {}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = setup_with_mock().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;

    // get the first request from the Mock server
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = setup_with_mock().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");

    let status: SubscriberState = saved.status.try_into().unwrap();
    assert_eq!(status, SubscriberState::Pending);
}
