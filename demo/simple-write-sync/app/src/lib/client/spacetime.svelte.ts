import {
    DbConnection,
    type Document,
    type ErrorContext,
    type EventContext
} from "$lib/module_bindings";
import {PUBLIC_SPACETIME_SERVER_MODULE_NAME, PUBLIC_SPACETIME_SERVER_URI} from "$env/static/public";
import type {Identity} from "@clockworklabs/spacetimedb-sdk";

// Custom rune factory function
export function createSpacetimeClient() {
    let connection = $state<DbConnection | null>(null);
    let documents = $state<Document[]>([]);

    function onConnect(conn: DbConnection) {
        console.log("Connected to SpacetimeDB server");

        conn.db.document.onInsert((_ctx: EventContext, doc: Document) => {
            console.log("Document inserted:", doc);
            documents.push(doc);
        })

        conn.db.document.onUpdate((_ctx: EventContext, _oldRow, doc: Document) => {
            console.log("Document updated:", doc);
            const index = documents.findIndex(d => d.id === doc.id);
            if (index !== -1) {
                documents = documents.map((item, i) => i === index ? doc : item);
            }
        })

        conn.db.document.onDelete((_ctx: EventContext, doc: Document) => {
            console.log("Document deleted:", doc);
            const index = documents.findIndex(d => d.id === doc.id);
            if (index !== -1) {
                documents.splice(index, 1);
            }
        })

        conn.subscriptionBuilder()
            .onApplied(() => {
                console.log("Subscription applied");
            })
            .onError((errorCtx) => {
                console.log("Subscription error:", errorCtx);
                documents.length = 0;
            })
            .subscribe("SELECT * FROM document");
    }

    function onDisconnect() {
        console.log("Disconnected from SpacetimeDB server");
        documents.length = 0;
    }

    function onConnectError (_ctx: ErrorContext, err: Error) {
        console.log('Error connecting to SpacetimeDB:', err);
    }

    // Return object with getters and methods
    return {
        // Reactive getters
        get connection() { return connection; },
        get documents() { return documents; },

        // Connection methods
        connect() {
            connection = DbConnection.builder()
                .withUri(PUBLIC_SPACETIME_SERVER_URI)
                .withModuleName(PUBLIC_SPACETIME_SERVER_MODULE_NAME)
                .withToken("")
                .onConnect(onConnect)
                .onDisconnect(onDisconnect)
                .onConnectError(onConnectError)
                .build();
        },

        disconnect() {
            connection?.disconnect();
            connection = null;
        },

        // Document operations
        createDocument(title: string) {
            if (!connection) return;
            connection.reducers.createDocument(title);
        },

        updateTitle(documentId: bigint, title: string) {
            if (!connection) return;
            connection.reducers.updateDocumentTitle(documentId, title);
        },

        // Row operations
        addRow(documentId: bigint) {
            if (!connection) return;
            connection.reducers.addRowToDocument(documentId);
        },

        addRowAfter(documentId: bigint, afterIndex: number, content: string = "") {
            if (!connection) return;
            connection.reducers.addRowAfter(documentId, afterIndex, content);
        },

        deleteRow(documentId: bigint, rowIndex: number) {
            if (!connection) return;
            connection.reducers.deleteRow(documentId, rowIndex);
        },

        mergeWithPreviousRow(documentId: bigint, rowIndex: number) {
            if (!connection) return;
            connection.reducers.mergeWithPreviousRow(documentId, rowIndex);
        },

        splitRow(documentId: bigint, rowIndex: number, splitPosition: number) {
            if (!connection) return;
            connection.reducers.splitRow(documentId, rowIndex, splitPosition);
        },

        // Editing operations
        startEditingRow(documentId: bigint, rowIndex: number) {
            if (!connection) return;
            connection.reducers.startEdit(documentId, rowIndex);
        },

        editRow(documentId: bigint, rowIndex: number, newValue: string) {
            if (!connection) return;
            connection.reducers.editRow(documentId, rowIndex, newValue);
        },

        stopEditingRow(documentId: bigint, rowIndex: number) {
            if (!connection) return;
            connection.reducers.stopEdit(documentId, rowIndex);
        },

        // Helper methods
        editorIsCurrentUser(editor: Identity): boolean {
            return connection ? editor.isEqual(connection.identity as Identity) : false;
        },

        getCurrentUser(): Identity | undefined {
            return connection?.identity;
        },

        // Utility method to check if a document exists and has rows
        hasDocument(documentId: bigint): boolean {
            return documents.some(d => d.id === documentId);
        },

        getDocument(documentId: bigint): Document | undefined {
            return documents.find(d => d.id === documentId);
        }
    };
}

// Create and export the client instance
export const spacetimeClient = createSpacetimeClient();