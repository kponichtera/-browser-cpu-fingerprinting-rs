use actix_web::{HttpResponse, Responder, get};
use actix_web::web::Data;
use crate::context::BackendContext;

/*
 TODO: Remove later
 */
#[get("/api/ping")]
pub async fn ping(context: Data<BackendContext>) -> impl Responder {
    let row: (i64,) = sqlx::query_as("SELECT $1")
        .bind(150_i64)
        .fetch_one(&context.connection_pool)
        .await
        .unwrap();

    HttpResponse::Ok()
        .body(row.0.to_string())
}