use actix_web::{HttpResponse, web};

#[derive(serde::Deserialize)]
pub(crate) struct FormData {
    email: String,
    name: String,
}

pub(crate) async fn subscribe(form: web::Form<FormData>) -> HttpResponse {
    println!("{}, {}", form.email, form.name);
    HttpResponse::Ok().finish()
}

// TODO: test the actual behavior of subscribe (i.e. that it inserts into the db, etc.)
