import React, {useState, useEffect, useCallback} from 'react';
import {
    Typography,
    Box,
    Paper,
    Table,
    TableBody,
    TableCell,
    TableContainer,
    TableHead,
    TableRow,
    CircularProgress,
    Alert,
    Button,
} from '@mui/material';
import api from '../services/api';
import CreateServerModal from '../components/CreateServerModal';
import ServerTableRow from '../components/ServerTableRow';
import useInterval from '../hooks/useInterval';

/**
 * @typedef {object} Server
 * @property {string} server_id
 * @property {string} ip_address
 * @property {string} status
 * @property {number | null} vm_id
 * @property {string | null} node_name
 */

const isTransient = (status) =>
    ['setting_up', 'deleting', 'starting', 'stopping', 'rebooting', 'shutting_down'].includes(status);

function DashboardPage() {
    /** @type {[Server[], function(Server[]): void]} */
    const [servers, setServers] = useState([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState('');
    const [isPolling, setIsPolling] = useState(false);
    const [isModalOpen, setIsModalOpen] = useState(false);

    const handleOpenModal = () => setIsModalOpen(true);

    const handleClose = (shouldRefresh = false) => {
        setIsModalOpen(false);
        if (shouldRefresh) {
            refreshServers();
            setIsPolling(true);
        }
    };

    const refreshServers = useCallback(() => {
        setLoading(true);
        api.get('/servers')
            .then(response => {
                setServers(response.data.result || []);
                if (!response.data.result?.some(s => isTransient(s.status))) {
                    setIsPolling(false);
                }
            })
            .catch(err => {
                setError(err.response?.data?.message || 'Failed to fetch servers.');
                setIsPolling(false);
            })
            .finally(() => {
                setLoading(false);
            });
    }, []);

    useEffect(() => {
        refreshServers();
    }, [refreshServers]);

    useInterval(() => {
        api.get('/servers')
            .then(response => {
                const fetchedServers = response.data.result || [];
                setServers(fetchedServers);
                if (!fetchedServers.some(s => isTransient(s.status))) {
                    setIsPolling(false);
                }
            })
            .catch(err => {
                console.error('Polling failed:', err);
                setError(err.response?.data?.message || 'Polling failed.');
                setIsPolling(false);
            });
    }, isPolling ? 5000 : null);


    const handleAction = async (serverId, action) => {
        const optimisticStatusMap = {
            start: 'starting',
            stop: 'stopping',
            reboot: 'rebooting',
            shutdown: 'shutting_down',
        };
        setServers(prev => prev.map(s => s.server_id === serverId ? {
            ...s,
            status: optimisticStatusMap[action]
        } : s));
        setIsPolling(true);

        try {
            await api.post(`/servers/${serverId}/actions`, {action});
        } catch (err) {
            console.error(`Failed to ${action} server:`, err);
            setError(err.response?.data?.message || `Failed to ${action} server.`);
            refreshServers(); // Re-fetch to get actual state on error
        }
    };

    const handleDelete = async (serverId) => {
        if (!window.confirm('Are you sure you want to delete this server? This action is irreversible.')) {
            return;
        }
        setServers(prev => prev.map(s => s.server_id === serverId ? {
            ...s,
            status: 'deleting'
        } : s));
        setIsPolling(true);

        try {
            await api.delete(`/servers/${serverId}`);
            // On successful deletion, we expect the server to be gone from the list.
            // The polling will handle updating the list.
        } catch (err) {
            console.error('Failed to delete server:', err);
            setError(err.response?.data?.message || 'Failed to delete server.');
            refreshServers(); // Re-fetch to get actual state on error
        }
    };

    return (
        <Box>
            <Box sx={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                mb: 2
            }}>
                <Typography variant="h4" component="h1">
                    Your Servers
                </Typography>
                <div>
                    <Button variant="contained" color="primary"
                            onClick={handleOpenModal} sx={{mr: 2}}>
                        Create Server
                    </Button>
                    <Button variant="contained" onClick={refreshServers}
                            disabled={loading}>
                        Refresh
                    </Button>
                </div>
            </Box>

            <CreateServerModal
                open={isModalOpen}
                handleClose={handleClose}
            />

            {loading && servers.length === 0 ? (
                <Box sx={{display: 'flex', justifyContent: 'center', mt: 4}}>
                    <CircularProgress/>
                </Box>
            ) : error ? (
                <Alert severity="error" sx={{mt: 2}}>{error}</Alert>
            ) : (
                <TableContainer component={Paper}>
                    <Table sx={{minWidth: 650}} aria-label="servers table">
                        <TableHead>
                            <TableRow>
                                <TableCell>IP Address</TableCell>
                                <TableCell>Status</TableCell>
                                <TableCell>VM ID</TableCell>
                                <TableCell>Node</TableCell>
                                <TableCell>Server ID</TableCell>
                                <TableCell>Actions</TableCell>
                            </TableRow>
                        </TableHead>
                        <TableBody>
                            {servers.length === 0 ? (
                                <TableRow>
                                    <TableCell colSpan={6} align="center">
                                        No servers found.
                                    </TableCell>
                                </TableRow>
                            ) : (
                                servers.map((server) => (
                                    <ServerTableRow
                                        key={server.server_id}
                                        server={server}
                                        onAction={handleAction}
                                        onDelete={handleDelete}
                                    />
                                ))
                            )}
                        </TableBody>
                    </Table>
                </TableContainer>
            )}
        </Box>
    );
}

export default DashboardPage;
