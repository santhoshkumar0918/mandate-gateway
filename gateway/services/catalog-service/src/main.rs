use axum::{routing::get, Json, Router};
use manifest::{Availability, MerchantManifest, Product};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

mod manifest;

/// Sample merchant manifest for demo.
fn sample_manifest() -> MerchantManifest {
    MerchantManifest {
        merchant_id: "merchant-001".into(),
        merchant_name: "TechStore Demo".into(),
        capability_version: "1.0".into(),
        supported_scopes: vec!["electronics".into(), "accessories".into()],
        catalog_endpoint: "http://localhost:8000/catalog".into(),
        mandate_endpoint: "http://localhost:8000/mandate".into(),
    }
}

/// Sample catalog for demo.
fn sample_catalog() -> Vec<Product> {
    vec![
        Product {
            product_id: "prod-001".into(),
            offer_id: "off-001".into(),
            title: "Wireless Mouse".into(),
            description: "Ergonomic wireless mouse with USB-C receiver".into(),
            category: "electronics".into(),
            price: 129_900,
            currency: "INR".into(),
            availability: Availability::InStock,
            inventory_count: 50,
            seller_name: "TechStore Demo".into(),
            seller_id: "merchant-001".into(),
            updated_at: chrono::Utc::now(),
        },
        Product {
            product_id: "prod-002".into(),
            offer_id: "off-002".into(),
            title: "USB-C Hub 7-in-1".into(),
            description: "HDMI, USB-A x3, SD, microSD, USB-C PD".into(),
            category: "accessories".into(),
            price: 249_900,
            currency: "INR".into(),
            availability: Availability::InStock,
            inventory_count: 30,
            seller_name: "TechStore Demo".into(),
            seller_id: "merchant-001".into(),
            updated_at: chrono::Utc::now(),
        },
    ]
}

async fn get_manifest() -> Json<MerchantManifest> {
    Json(sample_manifest())
}

async fn get_catalog() -> Json<Vec<Product>> {
    Json(sample_catalog())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let app = Router::new()
        .route("/manifest", get(get_manifest))
        .route("/catalog", get(get_catalog));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("catalog-service listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
