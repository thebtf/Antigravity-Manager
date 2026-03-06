import { useEffect, useRef } from 'react';
import { useConfigStore } from '../../stores/useConfigStore';
import { useAccountStore } from '../../stores/useAccountStore';

function BackgroundTaskRunner() {
    const { config } = useConfigStore();
    const { refreshAllQuotas } = useAccountStore();

    // Use refs to track previous state to detect "off -> on" transitions
    const prevAutoRefreshRef = useRef(false);
    const prevAutoSyncRef = useRef(false);

    // Auto Refresh Quota Effect
    useEffect(() => {
        if (!config) return;

        let intervalId: ReturnType<typeof setTimeout> | null = null;
        const { auto_refresh, refresh_interval } = config;

        // Check if we just turned it on
        if (auto_refresh && !prevAutoRefreshRef.current) {
            console.log('[BackgroundTask] Auto-refresh enabled, executing immediately...');
            refreshAllQuotas();
        }
        prevAutoRefreshRef.current = auto_refresh;

        if (auto_refresh && refresh_interval > 0) {
            // Jittered refresh: ±30% of configured interval to avoid predictable patterns
            const scheduleNext = () => {
                const baseMs = refresh_interval * 60 * 1000;
                const jitter = baseMs * 0.3;
                const delay = Math.min(baseMs + (Math.random() * 2 - 1) * jitter, 2147483647);
                console.log(`[BackgroundTask] Next auto-refresh in ${Math.round(delay / 1000 / 60)}min`);
                intervalId = setTimeout(() => {
                    console.log('[BackgroundTask] Auto-refreshing all quotas...');
                    refreshAllQuotas();
                    scheduleNext();
                }, delay);
            };
            scheduleNext();
        }

        return () => {
            if (intervalId) {
                console.log('[BackgroundTask] Clearing auto-refresh timer');
                clearTimeout(intervalId);
            }
        };
    }, [config?.auto_refresh, config?.refresh_interval]);

    // Auto Sync Current Account Effect
    useEffect(() => {
        if (!config) return;

        let intervalId: ReturnType<typeof setTimeout> | null = null;
        const { auto_sync, sync_interval } = config;
        const { syncAccountFromDb } = useAccountStore.getState();

        // Check if we just turned it on
        if (auto_sync && !prevAutoSyncRef.current) {
            console.log('[BackgroundTask] Auto-sync enabled, executing immediately...');
            syncAccountFromDb();
        }
        prevAutoSyncRef.current = auto_sync;

        if (auto_sync && sync_interval > 0) {
            // Jittered sync: ±30% of configured interval for consistency
            const scheduleNext = () => {
                const baseMs = sync_interval * 60 * 1000;
                const jitter = baseMs * 0.3;
                const delay = Math.min(baseMs + (Math.random() * 2 - 1) * jitter, 2147483647);
                intervalId = setTimeout(() => {
                    console.log('[BackgroundTask] Auto-syncing current account from DB...');
                    syncAccountFromDb();
                    scheduleNext();
                }, delay);
            };
            scheduleNext();
        }

        return () => {
            if (intervalId) {
                console.log('[BackgroundTask] Clearing auto-sync timer');
                clearTimeout(intervalId);
            }
        };
    }, [config?.auto_sync, config?.sync_interval]);

    // Render nothing
    return null;
}

export default BackgroundTaskRunner;
