extern crate serde_json;

#[derive(Serialize, Deserialize, Debug)]
pub struct Subscription {
    kind: String,
    product_ids: Vec<String>,
    channels: Vec<String>,
}

impl Subscription {
    pub fn new(products: Vec<String>) -> Subscription {
        return Subscription {
            kind: "subscribe".to_string(),
            product_ids: products,
            channels: vec!["full".to_string()],
        };
    }
}
