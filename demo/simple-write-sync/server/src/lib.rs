use spacetimedb::{Identity, ReducerContext, SpacetimeType, Table};
use spacetimedb::rand::Rng;

#[derive(SpacetimeType, Clone)]
pub struct DocumentRow {
    content: String,
    editor: Option<Identity>
}

#[spacetimedb::table(name = document, public)]
#[derive(Clone)]
pub struct Document {
    #[primary_key]
    id: u64,
    title: String,
    rows: Vec<DocumentRow>,
}

// Helper function to ensure a user can only edit one row at a time
fn clear_user_from_all_other_rows(
    ctx: &ReducerContext,
    user: Identity,
    except_document_id: Option<u64>,
    except_row_index: Option<usize>
) {
    for doc in ctx.db.document().iter() {
        let mut doc_modified = false;
        let mut updated_doc = doc.clone();

        for (i, row) in updated_doc.rows.iter_mut().enumerate() {
            if let Some(editor) = &row.editor {
                if *editor == user {
                    // Don't remove from the excepted row
                    let is_excepted_row = match (except_document_id, except_row_index) {
                        (Some(doc_id), Some(row_idx)) => doc.id == doc_id && i == row_idx,
                        _ => false,
                    };

                    if !is_excepted_row {
                        row.editor = None;
                        doc_modified = true;
                    }
                }
            }
        }

        if doc_modified {
            ctx.db.document().id().update(updated_doc);
        }
    }
}

// Helper function to set a user as editor of a specific row (after clearing others)
fn set_user_as_editor(
    ctx: &ReducerContext,
    document_id: u64,
    row_index: usize,
    user: Identity
) {
    // First clear user from all other rows
    clear_user_from_all_other_rows(ctx, user, Some(document_id), Some(row_index));

    // Then set user as editor of target row
    if let Some(mut document) = ctx.db.document().id().find(document_id) {
        if let Some(row) = document.rows.get_mut(row_index) {
            row.editor = Some(user);
            ctx.db.document().id().update(document);
        }
    }
}

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Called when the module is initially published
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    log::info!("Client connected: {}", ctx.sender);
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    let disconnected_identity = ctx.sender;
    clear_user_from_all_other_rows(ctx, disconnected_identity, None, None);
    log::info!("Cleaned up editor for disconnected identity: {disconnected_identity}");
}

#[spacetimedb::reducer]
pub fn create_document(ctx: &ReducerContext, title: String) {
    let id = ctx.rng().gen::<u64>();

    let document = Document {
        id,
        title: title.clone(),
        rows: vec![DocumentRow {
            content: String::new(),
            editor: None,
        }], // Start with one empty row
    };

    ctx.db.document().insert(document);
    log::info!("Created document {id} with title: {title}");
}

#[spacetimedb::reducer]
pub fn update_document_title(ctx: &ReducerContext, document_id: u64, new_title: String) {
    if let Some(mut document) = ctx.db.document().id().find(document_id) {
        document.title = new_title.clone();
        ctx.db.document().id().update(document);
        log::info!("Updated document {document_id} title to: {new_title}");
    }
}

#[spacetimedb::reducer]
pub fn add_row_to_document(ctx: &ReducerContext, document_id: u64) {
    if let Some(mut document) = ctx.db.document().id().find(document_id) {
        let new_row = DocumentRow {
            content: String::new(),
            editor: None,
        };

        document.rows.push(new_row);
        ctx.db.document().id().update(document);

        log::info!("Added empty row to document {document_id}");
    }
}

#[spacetimedb::reducer]
pub fn add_row_after(ctx: &ReducerContext, document_id: u64, after_index: u32, content: String) {
    if let Some(mut document) = ctx.db.document().id().find(document_id) {
        let new_row = DocumentRow {
            content,
            editor: None,
        };

        // Insert at the position after the specified index
        let insert_position = (after_index as usize + 1).min(document.rows.len());
        document.rows.insert(insert_position, new_row);

        ctx.db.document().id().update(document);
        log::info!("Added row after index {after_index} in document {document_id}");
    }
}

#[spacetimedb::reducer]
pub fn delete_row(ctx: &ReducerContext, document_id: u64, row_index: u32) {
    if let Some(mut document) = ctx.db.document().id().find(document_id) {
        let index = row_index as usize;

        // Don't delete if it's the only row
        if document.rows.len() > 1 && index < document.rows.len() {
            // Check if the user is allowed to delete (either not being edited or user is the editor)
            let can_delete = match document.rows.get(index) {
                Some(row) => match &row.editor {
                    None => true,
                    Some(editor) => *editor == ctx.sender,
                },
                None => false,
            };

            if can_delete {
                document.rows.remove(index);
                ctx.db.document().id().update(document);
                log::info!("Deleted row {row_index} from document {document_id}");
            } else {
                log::warn!("User {} cannot delete row {} in document {} - being edited by someone else",
                    ctx.sender, row_index, document_id);
            }
        } else {
            log::warn!("Cannot delete row {row_index} from document {document_id} - either out of range or last row");
        }
    }
}

#[spacetimedb::reducer]
pub fn merge_with_previous_row(ctx: &ReducerContext, document_id: u64, row_index: u32) {
    if let Some(mut document) = ctx.db.document().id().find(document_id) {
        let index = row_index as usize;

        // Can only merge if not the first row
        if index > 0 && index < document.rows.len() {
            // Check permissions for both rows
            let can_merge = {
                let current_row = &document.rows[index];
                let previous_row = &document.rows[index - 1];

                // Current row must be editable by user
                let can_edit_current = match &current_row.editor {
                    None => true,
                    Some(editor) => *editor == ctx.sender,
                };

                // Previous row must not be edited by someone else
                let can_edit_previous = match &previous_row.editor {
                    None => true,
                    Some(editor) => *editor == ctx.sender,
                };

                can_edit_current && can_edit_previous
            };

            if can_merge {
                // Get content from current row
                let current_content = document.rows[index].content.clone();

                // Append to previous row
                document.rows[index - 1].content.push_str(&current_content);

                // Remove current row
                document.rows.remove(index);

                // Update document first
                ctx.db.document().id().update(document);

                // Set the user as editor of the merged row (this will clear other edits)
                set_user_as_editor(ctx, document_id, index - 1, ctx.sender);

                log::info!("Merged row {row_index} with previous row in document {document_id}");
            } else {
                log::warn!("User {} cannot merge row {} in document {} - permission denied",
                    ctx.sender, row_index, document_id);
            }
        }
    }
}

#[spacetimedb::reducer]
pub fn split_row(ctx: &ReducerContext, document_id: u64, row_index: u32, split_position: u32) {
    if let Some(mut document) = ctx.db.document().id().find(document_id) {
        let index = row_index as usize;

        if index < document.rows.len() {
            let row = &document.rows[index];

            // Check if user can edit this row
            let can_edit = match &row.editor {
                Some(editor) => *editor == ctx.sender,
                None => false,
            };

            if can_edit {
                let content = &row.content;
                let split_pos = (split_position as usize).min(content.len());

                // Split the content
                let before = content[..split_pos].to_string();
                let after = content[split_pos..].to_string();

                // Update current row with content before split
                document.rows[index].content = before;
                document.rows[index].editor = None; // Release edit lock

                // Create new row with content after split
                let new_row = DocumentRow {
                    content: after,
                    editor: None,
                };

                // Insert new row after current
                document.rows.insert(index + 1, new_row);

                ctx.db.document().id().update(document);
                log::info!("Split row {row_index} at position {split_position} in document {document_id}");
            } else {
                log::warn!("User {} cannot split row {} in document {} - not the editor",
                    ctx.sender, row_index, document_id);
            }
        }
    }
}

#[spacetimedb::reducer]
pub fn start_edit(ctx: &ReducerContext, document_id: u64, row_index: u32) {
    if let Some(document) = ctx.db.document().id().find(document_id) {
        if let Some(target_row) = document.rows.get(row_index as usize) {
            // Check if the row is available for editing (no editor or user is already the editor)
            let can_start_edit = match &target_row.editor {
                None => true,
                Some(editor) => *editor == ctx.sender,
            };

            if can_start_edit {
                // Use helper to set user as editor (will clear other edits automatically)
                set_user_as_editor(ctx, document_id, row_index as usize, ctx.sender);
                log::info!("User {} started editing row {} in document {}", ctx.sender, row_index, document_id);
            } else {
                log::warn!("User {} cannot start editing row {} in document {} - already being edited by someone else", ctx.sender, row_index, document_id);
            }
        }
    }
}

#[spacetimedb::reducer]
pub fn edit_row(
    ctx: &ReducerContext,
    document_id: u64,
    row_index: u32,
    new_content: String
) {
    if let Some(mut document) = ctx.db.document().id().find(document_id) {
        if let Some(row) = document.rows.get_mut(row_index as usize) {
            // Check if user can edit (must be the current editor)
            let can_edit = match &row.editor {
                Some(editor) => *editor == ctx.sender,
                None => false, // No one is editing, so can't edit without starting edit first
            };

            if can_edit {
                // Update the content
                row.content = new_content;
                ctx.db.document().id().update(document);
                log::info!("User {} edited row {} in document {}", ctx.sender, row_index, document_id);
            } else {
                log::warn!("User {} denied editing row {} in document {} - not the current editor", ctx.sender, row_index, document_id);
            }
        }
    }
}

#[spacetimedb::reducer]
pub fn stop_edit(ctx: &ReducerContext, document_id: u64, row_index: u32) {
    if let Some(mut document) = ctx.db.document().id().find(&document_id) {
        if let Some(row) = document.rows.get_mut(row_index as usize) {
            // Only the current editor can stop editing
            if let Some(editor) = &row.editor {
                if *editor == ctx.sender {
                    row.editor = None;
                    ctx.db.document().id().update(document);
                    log::info!("User {} stopped editing row {} in document {}", ctx.sender, row_index, document_id);
                }
            }
        }
    }
}

#[spacetimedb::reducer]
pub fn stop_all_edits(ctx: &ReducerContext) {
    clear_user_from_all_other_rows(ctx, ctx.sender, None, None);
    log::info!("User {} stopped editing all rows", ctx.sender);
}