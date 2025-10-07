fn main() {
    println!("Migration Utility!");

    // Original: $2y$10$di2wHvDjMahfs2dUagJu/.udZ0v00WcJFxxUdK2gAZ6Qq0hrEbD9O
    let password = "Harry!Password";
    let whmcs_cost = 10;
    let hash = bcrypt::hash(password, whmcs_cost).unwrap();
    println!("hash: {}", hash);

    // Last: "$2b$10$OZMysKn3Z15BOcj5UHua7uBcgxkzvyZeTr5KiimhjY541Mk.Gn5wu";
    let valid = bcrypt::verify(password, &hash).unwrap();
    println!("valid: {}", valid);
}
