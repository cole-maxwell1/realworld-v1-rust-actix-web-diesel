use crate::app::article::model::Article;
use crate::app::user::model::User;
use crate::schema::favorites;
use chrono::NaiveDateTime;
use diesel::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Queryable, Identifiable, Associations, Clone, Debug)]
#[belongs_to(Article, foreign_key = "article_id")]
#[belongs_to(User, foreign_key = "user_id")]
#[table_name = "favorites"]
pub struct Favorite {
    pub id: Uuid,
    pub article_id: Uuid,
    pub user_id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Favorite {
    pub fn favorite(conn: &PgConnection, record: &FavorteAction) -> usize {
        let item = diesel::insert_into(favorites::table)
            .values(record)
            .execute(conn)
            .expect("could not do favorite.");

        item
    }

    pub fn unfavorite(conn: &PgConnection, params: &UnfavoriteAction) -> usize {
        use crate::schema::favorites;
        use crate::schema::favorites::dsl::*;
        let item = diesel::delete(favorites::table)
            .filter(favorites::user_id.eq_all(params.user_id))
            .filter(favorites::article_id.eq_all(params.article_id))
            .execute(conn)
            .expect("could not unfavorite.");
        item
    }
}

#[derive(Insertable)]
#[table_name = "favorites"]
pub struct FavorteAction {
    pub user_id: Uuid,
    pub article_id: Uuid,
}

pub struct UnfavoriteAction {
    pub user_id: Uuid,
    pub article_id: Uuid,
}
