/// Represents a client row in the `mysql` database.
///
#[derive(Debug, sqlx::FromRow)]
pub struct Client {
    pub id: i32,
    pub firstname: String,
    pub lastname: String,
    pub email: String,
    pub address1: String,
    pub city: String,
    pub state: String,
    pub postcode: String,
    pub country: String,
    pub phonenumber: String,
    pub password: String,
}
