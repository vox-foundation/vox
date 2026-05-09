import React, { useState, useEffect } from 'react';
import { loadCrashes, clearCrashes } from '../ErrorBoundary';

export default function SettingsPage() {
    const [crashes, setCrashes] = useState<object[]>([]);
    const [sent, setSent] = useState(false);

    useEffect(() => {
        loadCrashes().then(setCrashes);
    }, []);

    async function sendReports() {
        if (crashes.length === 0) return;
        try {
            await fetch('/api/record_crash_report', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ crashes }),
            });
            await clearCrashes();
            setCrashes([]);
            setSent(true);
        } catch {
            alert('Failed to send reports. Try again later.');
        }
    }

    return (
        <div style={{ padding: '1rem' }}>
            <h1>Settings</h1>
            <section>
                <h2>Crash Reports</h2>
                {crashes.length === 0 ? (
                    <p>No crash reports stored locally.</p>
                ) : (
                    <>
                        <p>{crashes.length} crash report(s) stored locally.</p>
                        <button onClick={sendReports}>Send to developers (opt-in)</button>
                    </>
                )}
                {sent && <p>Reports sent. Thank you!</p>}
            </section>
        </div>
    );
}
