use actix_cors::Cors;
use actix_multipart::form::MultipartFormConfig;
use actix_multipart::MultipartError;
use actix_web::get;
use actix_web::middleware::Logger;
use actix_web::Error;
use actix_web::HttpRequest;
use actix_web::http::header;
use actix_web::{web, App, HttpServer};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use env_logger::Env;

pub mod extraction;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod utils;

use middleware::api_key::ApiKeyMiddlewareFactory;
use routes::auth::create_api_key;
use routes::health::health_check;
use routes::task::{create_extraction_task, get_task_status};
use utils::db::deadpool_postgres;
use utils::storage::config_s3::create_client;
use utoipa::OpenApi;
use utoipa_redoc::{Redoc, Servable};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn run_migrations(url: &str) {
    use diesel::prelude::*;

    let mut conn = diesel::pg::PgConnection::establish(url).expect("Failed to connect to database");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");

    println!("Migrations run successfully");
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "ChunkMyDocs API",
        description = "API service for document layout analysis and chunking to convert document into RAG/LLM-ready data.", 
        contact(
            name = "Trieve x Lumina Teams",
            url = "https://trieve.ai",
            email = "developers@trieve.ai",
        ),
        license(
            name = "MIT",
            url = "https://github.com/lumina-ai-inc/chunk-my-docs/blob/main/LICENSE.txt",
        ),
        version = "0.0.0",
    ),
    servers(
        (url = "https://chunkmydocs.com",
        description = "Production server"),
        (url = "http://localhost:8000",
        description = "Local development server"),
    ),
    paths(
        routes::health::health_check,
        routes::task::create_extraction_task,
        routes::task::get_task_status,
    ),
    components(
        schemas(
            models::extraction::extract::UploadForm,
            models::extraction::task::TaskResponse,
            models::extraction::task::Status,
            models::extraction::extract::Model,
        ),
    ),
    tags(
        (name = "health", description = "Endpoint for checking the health of the service."),
    ),
)]
pub struct ApiDoc;

#[get("/openapi.json")]
pub async fn get_openapi_spec_handler() -> impl actix_web::Responder {
    web::Json(ApiDoc::openapi())
}

pub fn main() -> std::io::Result<()> {
    actix_web::rt::System::new().block_on(async move {
        let pg_pool = deadpool_postgres::create_pool();
        let s3_client = create_client().await.expect("Failed to create S3 client");
        run_migrations(&std::env::var("PG__URL").expect("PG__URL must be set in .env file"));

        fn handle_multipart_error(err: MultipartError, _: &HttpRequest) -> Error {
            println!("Multipart error: {}", err);
            Error::from(err)
        }
        let max_size: usize = std::env::var("MAX_TOTAL_LIMIT")
            .unwrap_or_else(|_| "10485760".to_string()) // Default to 10 MB if not set
            .parse()
            .expect("MAX_TOTAL_LIMIT must be a valid usize");
        let max_memory_size: usize = std::env::var("MAX_MEMORY_LIMIT")
            .unwrap_or_else(|_| "10485760".to_string()) // Default to 10 MB if not set
            .parse()
            .expect("MAX_MEMORY_LIMIT must be a valid usize");
        println!("Max size: {}", max_size);

        let timeout: usize = std::env::var("TIMEOUT")
            .unwrap_or_else(|_| "600".to_string())
            .parse()
            .expect("TIMEOUT must be a valid usize");
        let timeout = std::time::Duration::from_secs(timeout.try_into().unwrap());

        env_logger::init_from_env(Env::default().default_filter_or("info"));

        HttpServer::new(move || {
            let cors = Cors::default()
            .allowed_origin("http://localhost:5173")
            .allowed_origin("http://localhost:5174")
            .allowed_origin("https://www.chunkmydocs.com")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::HeaderName::from_static("x-api-key"),
            ])
            .max_age(3600);

            App::new()
                .wrap(Logger::default())
                .wrap(Logger::new("%a %{User-Agent}i"))
                .app_data(web::Data::new(pg_pool.clone()))
                .app_data(web::Data::new(s3_client.clone()))
                .app_data(
                    MultipartFormConfig::default()
                        .total_limit(max_size)
                        .memory_limit(max_memory_size)
                        .error_handler(handle_multipart_error),
                )
                .service(Redoc::with_url("/redoc", ApiDoc::openapi()))
                .route("/", web::get().to(health_check))
                .route("/api_key", web::post().to(create_api_key))
                .route("/health", web::get().to(health_check))
                .service(
                    web::scope("/api")
                        .wrap(ApiKeyMiddlewareFactory)
                        .route("/task", web::post().to(create_extraction_task))
                        .route("/task/{task_id}", web::get().to(get_task_status)),
                )
        })
        .bind("0.0.0.0:8000")?
        .keep_alive(timeout)
        .run()
        .await
    })
}
