const WHMCS_COST: u32 = 10;

fn main() {
    println!("Migration Utility!");

    // Original: $2y$10$di2wHvDjMahfs2dUagJu/.udZ0v00WcJFxxUdK2gAZ6Qq0hrEbD9O
    let password = "Harry!Password";
    let hash = bcrypt::hash(password, WHMCS_COST).unwrap();
    println!("hash: {}", hash);

    // Last: "$2b$10$OZMysKn3Z15BOcj5UHua7uBcgxkzvyZeTr5KiimhjY541Mk.Gn5wu";
    let valid = bcrypt::verify(password, &hash).unwrap();
    println!("valid: {}", valid);
}
