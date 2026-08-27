use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::manifest::{Availability, Product};

#[derive(Clone)]
pub struct CatalogStore {
    products: Arc<Mutex<HashMap<String, Product>>>,
}

impl CatalogStore {
    pub fn new() -> Self {
        let mut map = HashMap::new();
        for p in sample_catalog() {
            map.insert(p.product_id.clone(), p);
        }
        Self {
            products: Arc::new(Mutex::new(map)),
        }
    }

    pub fn get(&self, product_id: &str) -> Option<Product> {
        self.products.lock().ok()?.get(product_id).cloned()
    }

    pub fn list(&self) -> Vec<Product> {
        match self.products.lock() {
            Ok(guard) => {
                let mut v: Vec<Product> = guard.values().cloned().collect();
                v.sort_by(|a, b| a.product_id.cmp(&b.product_id));
                v
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn set_price(&self, product_id: &str, new_price: i64) -> Option<Product> {
        let mut guard = self.products.lock().ok()?;
        let product = guard.get_mut(product_id)?;
        product.price = new_price;
        product.updated_at = chrono::Utc::now();
        Some(product.clone())
    }
}

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
