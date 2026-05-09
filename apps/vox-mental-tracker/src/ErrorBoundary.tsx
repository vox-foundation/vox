import React, { Component, ErrorInfo, ReactNode } from 'react';

interface Props {
    children: ReactNode;
}

interface State {
    hasError: boolean;
}

const DB_NAME = 'vox-crashes';
const STORE_NAME = 'crashes';
const MAX_ENTRIES = 50;

async function saveCrashToIdb(error: Error, errorInfo: ErrorInfo, route: string) {
    const db = await openCrashDb();
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    const entry = {
        id: crypto.randomUUID(),
        error: error.message,
        stack: error.stack ?? '',
        componentStack: errorInfo.componentStack ?? '',
        ts: new Date().toISOString(),
        route,
    };
    const addReq = store.add(entry);
    addReq.onsuccess = () => {
        const countReq = store.count();
        countReq.onsuccess = () => {
            if (countReq.result > MAX_ENTRIES) {
                const cursor = store.openCursor();
                cursor.onsuccess = (e) => {
                    const c = (e.target as IDBRequest<IDBCursorWithValue>).result;
                    if (c) c.delete();
                };
            }
        };
    };
}

function openCrashDb(): Promise<IDBDatabase> {
    return new Promise((resolve, reject) => {
        const req = indexedDB.open(DB_NAME, 1);
        req.onupgradeneeded = () => {
            req.result.createObjectStore(STORE_NAME, { keyPath: 'id' });
        };
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

export async function loadCrashes(): Promise<object[]> {
    const db = await openCrashDb();
    return new Promise((resolve, reject) => {
        const tx = db.transaction(STORE_NAME, 'readonly');
        const req = tx.objectStore(STORE_NAME).getAll();
        req.onsuccess = () => resolve(req.result);
        req.onerror = () => reject(req.error);
    });
}

export async function clearCrashes() {
    const db = await openCrashDb();
    const tx = db.transaction(STORE_NAME, 'readwrite');
    tx.objectStore(STORE_NAME).clear();
}

export class ErrorBoundary extends Component<Props, State> {
    constructor(props: Props) {
        super(props);
        this.state = { hasError: false };
    }

    static getDerivedStateFromError(): State {
        return { hasError: true };
    }

    componentDidCatch(error: Error, errorInfo: ErrorInfo) {
        const route = window.location.pathname;
        saveCrashToIdb(error, errorInfo, route).catch(console.error);
    }

    render() {
        if (this.state.hasError) {
            return (
                <div style={{ padding: '2rem', textAlign: 'center' }}>
                    <h2>Something went wrong.</h2>
                    <p>The error has been logged locally. Visit <a href="/settings">Settings</a> to send a report.</p>
                    <button onClick={() => this.setState({ hasError: false })}>Try again</button>
                </div>
            );
        }
        return this.props.children;
    }
}
