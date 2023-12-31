use crate::helpers::{setup_with_mock, spawn_app};
use zero2prod::domain::SubscriberState;

#[tokio::test]
async fn confirmations_without_tokens_rejected_with_400() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn link_returned_by_subscribe_returns_200() {
    let app = setup_with_mock().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = setup_with_mock().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");

    let status: SubscriberState = saved.status.try_into().unwrap();
    assert_eq!(status, SubscriberState::Active);
}

#[tokio::test]
async fn active_subscriber_does_not_receive_email() {
    let app = setup_with_mock().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;

    // get the first request from the Mock server
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let response = app.post_subscriptions(body.into()).await;
    assert_eq!(response.status().as_u16(), 200);

    // get the second request from the Mock server
    let email_request = &app.email_server.received_requests().await.unwrap();
    assert_eq!(email_request.len(), 1);
}

#[tokio::test]
async fn confirmation_with_nonexisting_token_return_401() {
    let app = setup_with_mock().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    let token = confirmation_links.html.query_pairs().next().unwrap().1;
    sqlx::query!(
        "DELETE from subscription_tokens WHERE subscription_token = $1",
        &token
    )
    .execute(&app.db_pool)
    .await
    .expect("Failed to delete token");

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(response.status().as_u16(), 401);
}

#[tokio::test]
async fn confirmation_with_bad_token_return_400() {
    let app = setup_with_mock().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);

    let mut url = confirmation_links.html.clone();
    url.query_pairs_mut().clear();
    url.query_pairs_mut()
        .append_pair("subscription_token", "TdBMIE4nBbp43AY1YDTr5XrCw6R15=");

    let response = reqwest::get(url).await.unwrap();

    assert_eq!(response.status().as_u16(), 400);
}
