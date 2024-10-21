#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Data {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Value {
    pub name: String,
    pub value: f64,
    pub uncertainity: f64,
}

impl Value {
    pub fn new(name: String, value: f64, uncertainity: f64) -> Self {
        Value { name, value, uncertainity }
    }
    
}