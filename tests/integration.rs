use rusqlite::{params, Connection};
use rusqlite_mapper::{FromRow, SqliteValue, ToRow};

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct Todo {
    id: i32,
    text: String,
    #[rusqlite(flatten, prefix = "author_")]
    author: User,
    #[rusqlite(flatten, prefix = "editor_")]
    editor: User,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct User {
    id: i32,
    name: String,
    #[rusqlite(flatten, prefix = "role_")]
    role: Option<Role>,
}

#[allow(dead_code)]
#[derive(Debug, FromRow)]
pub struct Role {
    id: i32,
    kind: String,
}

#[test]
fn from_row() {
    let connection = Connection::open_in_memory().unwrap();

    connection
        .execute_batch(
            "

            CREATE TABLE role (
                id INTEGER PRIMARY KEY,
                kind TEXT NOT NULL
            );

            CREATE TABLE user (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                role_id INTEGER NULL REFERENCES role(id)
            );

            CREATE TABLE todo (
                id INTEGER PRIMARY KEY,
                text TEXT NOT NULL,
                author_id INTEGER NOT NULL REFERENCES user(id),
                editor_id INTEGER NOT NULL REFERENCES user(id)
            );
            ",
        )
        .unwrap();

    let role_id: i32 = connection
        .prepare("INSERT INTO role(kind) VALUES ('admin') RETURNING id")
        .unwrap()
        .query_row(params![], |r| r.get(0))
        .unwrap();

    let user_ids = connection
        .prepare("INSERT INTO user(name, role_id) VALUES ('john', ?1), ('jack', null) RETURNING id")
        .unwrap()
        .query_map([role_id], |r| r.get(0))
        .unwrap()
        .collect::<Result<Vec<i32>, _>>()
        .unwrap();

    let todo_id: i32 = connection
        .prepare(
            "INSERT INTO todo(text, author_id, editor_id) VALUES ('laundry', ?1, ?2) RETURNING id",
        )
        .unwrap()
        .query_row(params![user_ids[0], user_ids[1]], |r| r.get(0))
        .unwrap();

    let todo = connection
        .query_row(
            "
            SELECT
                t.id,
                t.text,
                a.id as author_id,
                a.name as author_name,
                ar.id as author_role_id,
                ar.kind as author_role_kind,
                e.id as editor_id,
                e.name as editor_name,
                er.id as editor_role_id,
                er.kind as editor_role_kind
            FROM
                todo t
            JOIN user a ON
                a.id = t.author_id
            LEFT JOIN role ar ON
                a.role_id = ar.id
            JOIN user e ON
                e.id = t.editor_id
            LEFT JOIN role er ON
                e.role_id = er.id
            WHERE
                t.id = ?1",
            params![todo_id],
            Todo::try_from_row,
        )
        .unwrap();

    println!("{:#?}", todo);
}

// -=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-

#[derive(Debug, FromRow, ToRow)]
struct Person {
    #[rusqlite(primary_key)]
    id: i32,
    name: String,
    role: PersonRole,
    #[allow(dead_code)]
    #[rusqlite(skip)]
    ignore: IgnoreMe,
    data: Option<Vec<u8>>,
}

#[derive(Debug, SqliteValue)]
#[rusqlite_value(string)]
enum PersonRole {
    Admin,
    User,
}

#[derive(Debug, Default)]
struct IgnoreMe;

impl std::str::FromStr for PersonRole {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "admin" => Ok(PersonRole::Admin),
            "user" => Ok(PersonRole::User),
            _ => todo!(),
        }
    }
}

impl std::fmt::Display for PersonRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersonRole::Admin => write!(f, "admin"),
            PersonRole::User => write!(f, "user"),
        }
    }
}

#[test]
fn to_row() {
    let conn = rusqlite::Connection::open_in_memory().expect("Failed to open in memory database");

    conn.execute(dbg!(Person::create_table_statement().as_str()), ())
        .expect("Failed to create table");

    let me = Person {
        id: 0,
        role: PersonRole::Admin,
        ignore: IgnoreMe,
        name: "Steven".to_string(),
        data: None,
    };

    conn.execute(dbg!(&Person::insert_stmt()), me.to_params())
        .expect("Failed to insert");

    let mut stmt = conn
        .prepare("SELECT * FROM person")
        .expect("Failed to prepare statement");
    let person_iter = stmt
        .query_map([], Person::try_from_row)
        .expect("Failed to query map");

    for person in person_iter {
        println!("Found person {:?}", person.unwrap());
    }
}
