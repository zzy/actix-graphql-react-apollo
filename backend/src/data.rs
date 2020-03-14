use diesel::pg::PgConnection;
use diesel::prelude::*;
use juniper::{graphql_value, FieldError, FieldResult};

use super::schema::users::dsl::*;
use super::models::{ User, NewUser, CreateUserInput};

// This struct is basically a query manager. All the methods that it
// provides are static, making it a convenient abstraction for
// interacting with the database.
pub struct Users;

// Note that all the function names here map directly onto the function names
// associated with the QueryRoot and MutationRoot structs. This is NOT necessary
// but I personally prefer it.
impl Users {
    pub fn all_users(conn: &PgConnection) -> FieldResult<Vec<User>> {
        let res = users.load::<User>(conn);

        graphql_translate(res)
    }

    pub fn create_user(
        conn: &PgConnection,
        new_user: CreateUserInput,
    ) -> FieldResult<User> {
        use super::schema::users;

        let new_user = NewUser {
            email: &new_user.email,
            username: &new_user.username,
            password: &new_user.password,
            banned: &new_user.banned.unwrap_or(false), // Default value is false
        };

        let res = diesel::insert_into(users::table)
            .values(&new_user)
            .get_result(conn);

        graphql_translate(res)
    }

    pub fn get_user_by_id(
        conn: &PgConnection,
        user_id: i32,
    ) -> FieldResult<Option<User>> {
        match users.find(user_id).get_result::<User>(conn) {
            Ok(user) => Ok(Some(user)),
            Err(e) => match e {
                // Without this translation, GraphQL will return an error rather
                // than the more semantically sound JSON null if no User is found.
                diesel::result::Error::NotFound => FieldResult::Ok(None),
                _ => FieldResult::Err(FieldError::from(e)),
            },
        }
    }

    pub fn  banned_users(conn: &PgConnection) -> FieldResult<Vec<User>> {
        let res = users.filter(banned.eq(true)).load::<User>(conn);

        graphql_translate(res)
    }

    pub fn not_banned_users(conn: &PgConnection) -> FieldResult<Vec<User>> {
        let res = users.filter(banned.eq(false)).load::<User>(conn);

        graphql_translate(res)
    }

    pub fn mark_user_as_banned(
        conn: &PgConnection, 
        user_id: i32
    ) -> FieldResult<User> {
        mark_user_as(conn, user_id, true)
    }

    pub fn mark_user_as_not_banned(
        conn: &PgConnection,
        user_id: i32,
    ) -> FieldResult<User> {
        mark_user_as(conn, user_id, false)
    }
}

fn graphql_translate<T>(res: Result<T, diesel::result::Error>) -> FieldResult<T> {
    match res {
        Ok(t) => Ok(t),
        Err(e) => FieldResult::Err(FieldError::from(e)),
    }
}

// This helper function ensures that users don't mark Users as banned that are already banned
// (or not banned that are already not banned).
fn mark_user_as(
    conn: &PgConnection, 
    user_id: i32, 
    is_banned: bool
) -> FieldResult<User> {
    let res = users.find(user_id).get_result::<User>(conn);

    // Poor man's Ternary operator for error output text
    let msg = if is_banned { "banned" } else { "not banned" };

    match res {
        Ok(user) => {
            if user.banned == is_banned {
                let err = FieldError::new(
                    format!("User already marked as {}", msg),
                    // TODO: better error output
                    graphql_value!({ "cannot_update": "confict"}),
                );
                FieldResult::Err(err)
            }
            else {
                let res = diesel::update(users.find(user_id))
                    .set(banned.eq(is_banned))
                    .get_result::<User>(conn);
                graphql_translate(res)
            }
        }
        Err(e) => FieldResult::Err(FieldError::from(e)),
    }
}
