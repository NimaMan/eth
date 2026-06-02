use sqlx::PgPool;

use crate::Result;

pub const CREATE_TOKEN_PNL_SQL: &str = include_str!("../migrations/001_create_token_pnl.sql");

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    for statement in split_sql_statements(CREATE_TOKEN_PNL_SQL) {
        if statement.is_empty() {
            continue;
        }
        sqlx::query(&statement).execute(pool).await?;
    }
    Ok(())
}

fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut start = 0usize;
    let mut in_single_quote = false;
    let mut dollar_tag: Option<String> = None;
    let mut iter = sql.char_indices().peekable();

    while let Some((idx, ch)) = iter.next() {
        if let Some(tag) = dollar_tag.as_deref() {
            if sql[idx..].starts_with(tag) {
                for _ in 1..tag.len() {
                    iter.next();
                }
                dollar_tag = None;
            }
            continue;
        }

        if in_single_quote {
            if ch == '\'' {
                if matches!(iter.peek(), Some((_, '\''))) {
                    iter.next();
                } else {
                    in_single_quote = false;
                }
            }
            continue;
        }

        match ch {
            '\'' => in_single_quote = true,
            '$' => {
                if let Some(tag) = dollar_quote_tag(&sql[idx..]) {
                    for _ in 1..tag.len() {
                        iter.next();
                    }
                    dollar_tag = Some(tag);
                }
            }
            ';' => {
                let statement = sql[start..idx].trim();
                if !statement.is_empty() {
                    statements.push(statement.to_string());
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let statement = sql[start..].trim();
    if !statement.is_empty() {
        statements.push(statement.to_string());
    }
    statements
}

fn dollar_quote_tag(input: &str) -> Option<String> {
    let rest = input.strip_prefix('$')?;
    let end = rest.find('$')?;
    let tag = &rest[..end];
    if tag
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        Some(format!("${tag}$"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::split_sql_statements;

    #[test]
    fn keeps_dollar_quoted_blocks_intact() {
        let statements = split_sql_statements(
            r#"
            CREATE TABLE example(id BIGINT);
            DO $$
            BEGIN
                IF true THEN
                    ALTER TABLE example ADD COLUMN value BIGINT;
                END IF;
            END $$;
            SELECT 'literal;semicolon';
            "#,
        );

        assert_eq!(statements.len(), 3);
        assert!(statements[1].contains("ALTER TABLE example ADD COLUMN value BIGINT;"));
        assert_eq!(statements[2], "SELECT 'literal;semicolon'");
    }
}
