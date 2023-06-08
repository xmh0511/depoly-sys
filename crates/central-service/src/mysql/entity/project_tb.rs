//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "project_tb")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(column_type = "Text")]
    pub path: String,
    #[sea_orm(column_type = "Text")]
    pub name: String,
    pub parent_id: i32,
    #[sea_orm(column_type = "Text", nullable)]
    pub command: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub token: String,
    pub create_time: Option<DateTime>,
    pub update_time: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
