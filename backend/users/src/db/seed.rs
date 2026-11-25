use sqlx::PgPool;

use crate::auth::password;
use crate::db::user_queries;

// Creates the admin from ADMIN_EMAIL and ADMIN_PASSWORD if it does not exist yet.
// If the user already exists nothing changes (also not the password).
pub async fn seed_admin(pool: &PgPool, admin_email: &str, admin_password: &str) {
    let email = admin_email.trim().to_lowercase();
    let existing_user = user_queries::get_user_by_email(pool, &email)
        .await
        .expect("could not check the admin user");
    if existing_user.is_some() {
        return;
    }

    let password_hash = password::hash_password(admin_password);
    user_queries::insert_user(pool, "Admin", &email, &password_hash, "admin")
        .await
        .expect("could not create the admin user");
    tracing::info!("created admin user {email}");
}
