use std::collections::{HashMap, HashSet};

use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::database::{Database, Error};

type Result<T> = std::result::Result<T, Error>;
const NOW: &str = "CAST(unixepoch('subsec') * 1000 AS INTEGER)";

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupKind {
    Category,
    Tag,
    Collection,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberAction {
    Up,
    Down,
    Remove,
}

impl GroupKind {
    fn table(self) -> &'static str {
        match self {
            Self::Category => "categories",
            Self::Tag => "tags",
            Self::Collection => "collections",
        }
    }
    fn link(self) -> (&'static str, &'static str) {
        match self {
            Self::Category => ("book_categories", "category_id"),
            Self::Tag => ("book_tags", "tag_id"),
            Self::Collection => ("collection_books", "collection_id"),
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum CollectionSort {
    #[default]
    Manual,
    Title,
    Added,
}
impl CollectionSort {
    fn key(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Title => "title",
            Self::Added => "added",
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupInput {
    pub id: Option<i64>,
    pub revision: Option<i64>,
    pub name: String,
    pub parent_id: Option<i64>,
    pub description: Option<String>,
    pub sort_mode: Option<CollectionSort>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub description: Option<String>,
    pub sort_mode: String,
    pub revision: i64,
    pub count: i64,
}
#[derive(Serialize)]
pub struct Organization {
    pub categories: Vec<Group>,
    pub tags: Vec<Group>,
    pub collections: Vec<Group>,
    pub languages: Vec<String>,
}

fn constraint(error: rusqlite::Error) -> Error {
    if matches!(&error, rusqlite::Error::SqliteFailure(code, _) if code.code == rusqlite::ErrorCode::ConstraintViolation)
    {
        Error::OrganizationConflict
    } else {
        error.into()
    }
}

pub(super) fn check_group(
    tx: &Transaction<'_>,
    kind: GroupKind,
    id: i64,
    expected: i64,
) -> Result<()> {
    let current = tx
        .query_row(
            &format!("SELECT revision FROM {} WHERE id=?1", kind.table()),
            [id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .ok_or(Error::CatalogNotFound)?;
    if current != expected {
        return Err(Error::CatalogConflict);
    }
    Ok(())
}

pub(super) fn group_ids(
    connection: &rusqlite::Connection,
    kind: GroupKind,
    book: i64,
) -> rusqlite::Result<Vec<i64>> {
    let (table, column) = kind.link();
    let mut query = connection.prepare(&format!(
        "SELECT {column} FROM {table} WHERE book_id=?1 ORDER BY {column}"
    ))?;
    let rows = query.query_map([book], |row| row.get(0))?;
    rows.collect()
}

pub(super) fn assign_groups(
    tx: &Transaction<'_>,
    kind: GroupKind,
    book: i64,
    ids: &[i64],
) -> Result<()> {
    let mut unique = HashSet::new();
    if ids.len() > 5000 || ids.iter().any(|id| *id <= 0 || !unique.insert(*id)) {
        return Err(Error::InvalidCatalogInput);
    }
    let (table, column) = kind.link();
    for id in ids {
        let exists: bool = tx.query_row(
            &format!("SELECT EXISTS(SELECT 1 FROM {} WHERE id=?1)", kind.table()),
            [id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(Error::CatalogNotFound);
        }
    }
    let old = group_ids(tx, kind, book)?;
    for id in old.iter().filter(|id| !ids.contains(id)) {
        tx.execute(
            &format!("DELETE FROM {table} WHERE book_id=?1 AND {column}=?2"),
            params![book, id],
        )?;
        if matches!(kind, GroupKind::Collection) {
            tx.execute(
                &format!("UPDATE collections SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
                [id],
            )?;
        }
    }
    for id in ids.iter().filter(|id| !old.contains(id)) {
        if matches!(kind, GroupKind::Collection) {
            tx.execute(&format!(
                "INSERT INTO collection_books(collection_id,book_id,position,added_at)
                 VALUES (?1,?2,(SELECT coalesce(max(position),-1)+1 FROM collection_books WHERE collection_id=?1),{NOW})"
            ), params![id,book])?;
            tx.execute(
                &format!("UPDATE collections SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
                [id],
            )?;
        } else {
            tx.execute(
                &format!("INSERT INTO {table}(book_id,{column}) VALUES (?1,?2)"),
                params![book, id],
            )?;
        }
    }
    Ok(())
}

impl Database {
    pub fn edit_collection_member(
        &self,
        id: i64,
        revision: i64,
        book: i64,
        action: MemberAction,
    ) -> Result<()> {
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        check_group(&tx, GroupKind::Collection, id, revision)?;
        let position: i64 = tx
            .query_row(
                "SELECT position FROM collection_books WHERE collection_id=?1 AND book_id=?2",
                params![id, book],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(Error::CatalogNotFound)?;
        if matches!(action, MemberAction::Remove) {
            tx.execute(
                "DELETE FROM collection_books WHERE collection_id=?1 AND book_id=?2",
                params![id, book],
            )?;
            tx.execute(
                &format!("UPDATE books SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
                [book],
            )?;
        } else {
            let (comparison, order) = match action {
                MemberAction::Up => ("<", "DESC"),
                _ => (">", "ASC"),
            };
            let neighbor: Option<(i64,i64)> = tx.query_row(&format!(
                "SELECT book_id,position FROM collection_books WHERE collection_id=?1 AND position{comparison}?2
                 ORDER BY position {order} LIMIT 1"
            ),params![id,position],|row|Ok((row.get(0)?,row.get(1)?))).optional()?;
            if let Some((other, other_position)) = neighbor {
                tx.execute(
                    "UPDATE collection_books SET position=(SELECT max(position)+1 FROM collection_books WHERE collection_id=?1)
                     WHERE collection_id=?1 AND book_id=?2",params![id,book],
                )?;
                tx.execute(
                    "UPDATE collection_books SET position=?3 WHERE collection_id=?1 AND book_id=?2",
                    params![id, other, position],
                )?;
                tx.execute(
                    "UPDATE collection_books SET position=?3 WHERE collection_id=?1 AND book_id=?2",
                    params![id, book, other_position],
                )?;
            }
        }
        tx.execute(
            &format!("UPDATE collections SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
            [id],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn organization(&self) -> Result<Organization> {
        let tx = self.0.unchecked_transaction()?;
        let mut groups = Vec::new();
        for kind in [GroupKind::Category, GroupKind::Tag, GroupKind::Collection] {
            let (link, column) = kind.link();
            let extra = match kind {
                GroupKind::Category => "g.parent_id,NULL,'manual'",
                GroupKind::Tag => "NULL,NULL,'manual'",
                GroupKind::Collection => "NULL,g.description,g.sort_mode",
            };
            let mut query = tx.prepare(&format!(
                "SELECT g.id,g.name,{extra},g.revision,(SELECT count(*) FROM {link} WHERE {column}=g.id)
                 FROM {} g ORDER BY g.name_key,g.id", kind.table(),
            ))?;
            groups.push(
                query
                    .query_map([], |row| {
                        Ok(Group {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            parent_id: row.get(2)?,
                            description: row.get(3)?,
                            sort_mode: row.get(4)?,
                            revision: row.get(5)?,
                            count: row.get(6)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?,
            );
        }
        let languages = {
            let mut query = tx.prepare(
                "SELECT DISTINCT language FROM books WHERE language IS NOT NULL ORDER BY language",
            )?;
            let rows = query.query_map([], |row| row.get(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        let mut groups = groups.into_iter();
        let organization = Organization {
            categories: groups.next().unwrap_or_default(),
            tags: groups.next().unwrap_or_default(),
            collections: groups.next().unwrap_or_default(),
            languages,
        };
        tx.commit()?;
        Ok(organization)
    }

    pub fn save_group(&self, kind: GroupKind, input: &GroupInput) -> Result<i64> {
        let name = input.name.trim();
        if name.is_empty()
            || name.len() > 256
            || name.contains('\0')
            || input
                .description
                .as_ref()
                .is_some_and(|text| text.len() > 4096 || text.contains('\0'))
        {
            return Err(Error::InvalidCatalogInput);
        }
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        if let Some(id) = input.id {
            check_group(
                &tx,
                kind,
                id,
                input.revision.ok_or(Error::InvalidCatalogInput)?,
            )?;
        }
        if let (GroupKind::Category, Some(parent)) = (kind, input.parent_id) {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1)",
                [parent],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(Error::CatalogNotFound);
            }
            if let Some(id) = input.id {
                let cycle: bool = tx.query_row(
                    "WITH RECURSIVE ancestors(id) AS (
                     SELECT ?1 UNION SELECT c.parent_id FROM categories c JOIN ancestors a ON c.id=a.id WHERE c.parent_id IS NOT NULL)
                     SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=?2)", params![parent,id], |row| row.get(0),
                )?;
                if cycle {
                    return Err(Error::OrganizationConflict);
                }
            }
        }
        let key = super::search::normalize(name);
        let id = match (kind,input.id) {
            (GroupKind::Category, Some(id)) => {
                tx.execute("UPDATE categories SET name=?2,name_key=?3,parent_id=?4,revision=revision+1 WHERE id=?1",
                    params![id,name,key,input.parent_id]).map_err(constraint)?;
                id
            },
            (GroupKind::Category, None) => tx.query_row(
                "INSERT INTO categories(name,name_key,parent_id) VALUES (?1,?2,?3) RETURNING id",
                params![name,key,input.parent_id], |row| row.get(0)).map_err(constraint)?,
            (GroupKind::Tag, Some(id)) => {
                tx.execute("UPDATE tags SET name=?2,name_key=?3,revision=revision+1 WHERE id=?1",
                    params![id,name,key]).map_err(constraint)?;
                id
            },
            (GroupKind::Tag, None) => tx.query_row("INSERT INTO tags(name,name_key) VALUES (?1,?2) RETURNING id",
                params![name,key], |row| row.get(0)).map_err(constraint)?,
            (GroupKind::Collection, Some(id)) => {
                tx.execute(&format!("UPDATE collections SET name=?2,name_key=?3,description=?4,sort_mode=?5,revision=revision+1,updated_at={NOW} WHERE id=?1"),
                    params![id,name,key,input.description,input.sort_mode.unwrap_or_default().key()]).map_err(constraint)?;
                id
            },
            (GroupKind::Collection, None) => tx.query_row(&format!(
                "INSERT INTO collections(name,name_key,description,sort_mode,created_at,updated_at) VALUES (?1,?2,?3,?4,{NOW},{NOW}) RETURNING id"),
                params![name,key,input.description,input.sort_mode.unwrap_or_default().key()], |row| row.get(0)).map_err(constraint)?,
        };
        tx.commit()?;
        Ok(id)
    }

    pub fn delete_group(&self, kind: GroupKind, id: i64, revision: i64) -> Result<()> {
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        check_group(&tx, kind, id, revision)?;
        if matches!(kind, GroupKind::Category) {
            let children: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE parent_id=?1)",
                [id],
                |row| row.get(0),
            )?;
            if children {
                return Err(Error::CategoryHasChildren);
            }
        }
        let (link, column) = kind.link();
        tx.execute(&format!(
            "UPDATE books SET revision=revision+1,updated_at={NOW} WHERE id IN (SELECT book_id FROM {link} WHERE {column}=?1)"
        ), [id])?;
        tx.execute(&format!("DELETE FROM {} WHERE id=?1", kind.table()), [id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_collection_books(&self, id: i64, revision: i64, books: &[i64]) -> Result<()> {
        let mut unique = HashSet::new();
        if books.len() > 100_000 || books.iter().any(|id| *id <= 0 || !unique.insert(*id)) {
            return Err(Error::InvalidCatalogInput);
        }
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        check_group(&tx, GroupKind::Collection, id, revision)?;
        let old: HashMap<i64, i64> = {
            let mut query =
                tx.prepare("SELECT book_id,added_at FROM collection_books WHERE collection_id=?1")?;
            let rows = query.query_map([id], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        tx.execute("DELETE FROM collection_books WHERE collection_id=?1", [id])?;
        for (position, book) in books.iter().enumerate() {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM books WHERE id=?1)",
                [book],
                |row| row.get(0),
            )?;
            if !exists {
                return Err(Error::CatalogNotFound);
            }
            tx.execute(&format!(
                "INSERT INTO collection_books(collection_id,book_id,position,added_at) VALUES (?1,?2,?3,coalesce(?4,{NOW}))"
            ),params![id,book,position as i64,old.get(book)])?;
        }
        for book in old
            .keys()
            .copied()
            .chain(books.iter().copied())
            .collect::<HashSet<_>>()
        {
            tx.execute(
                &format!("UPDATE books SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
                [book],
            )?;
        }
        tx.execute(
            &format!("UPDATE collections SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
            [id],
        )?;
        tx.commit()?;
        Ok(())
    }
}
