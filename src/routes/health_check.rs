use actix_web::HttpResponse;

#[tracing::instrument(name = "Health Check")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
