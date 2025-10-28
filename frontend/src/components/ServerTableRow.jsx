import React from 'react';
import {
    TableRow,
    TableCell,
    Chip,
    Button,
    ButtonGroup,
    Box
} from '@mui/material';

/**
 * @param {string} status
 * @returns {"success" | "default" | "error" | "warning" | "primary"}
 */
const getStatusChipColor = (status) => {
    switch (status) {
        case 'running':
            return 'success';
        case 'stopped':
            return 'default';
        case 'failed':
            return 'error';
        case 'setting_up':
        case 'starting':
        case 'rebooting':
        case 'shutting_down':
        case 'deleting':
            return 'warning';
        default:
            return 'primary';
    }
};

const isTransient = (status) =>
    ['setting_up', 'deleting', 'starting', 'stopping', 'rebooting', 'shutting_down'].includes(status);

const ServerTableRow = ({server, onAction, onDelete}) => {
    return (
        <TableRow>
            <TableCell component="th" scope="row">
                {server.ip_address || 'N/A'}
            </TableCell>
            <TableCell>
                <Chip
                    label={server.status.replace('_', ' ')}
                    color={getStatusChipColor(server.status)}
                    size="small"
                    sx={{textTransform: 'capitalize'}}
                />
            </TableCell>
            <TableCell>{server.vm_id || 'N/A'}</TableCell>
            <TableCell>{server.node_name || 'N/A'}</TableCell>
            <TableCell>
                <code style={{
                    fontFamily: 'monospace',
                    backgroundColor: 'rgba(0, 0, 0, 0.05)',
                    padding: '2px 4px',
                    borderRadius: '4px',
                }}>
                    {server.server_id}
                </code>
            </TableCell>
            <TableCell>
                <Box sx={{display: 'flex', justifyContent: 'space-between'}}>
                    <ButtonGroup variant="contained" size="small">
                        <Button
                            color="primary"
                            onClick={() => onAction(server.server_id, 'start')}
                            disabled={server.status === 'running' || isTransient(server.status)}
                        >
                            Start
                        </Button>
                        <Button
                            color="warning"
                            onClick={() => onAction(server.server_id, 'stop')}
                            disabled={!['running', 'failed'].includes(server.status) || isTransient(server.status)}
                        >
                            Stop
                        </Button>
                        <Button
                            color="warning"
                            onClick={() => onAction(server.server_id, 'reboot')}
                            disabled={server.status !== 'running' || isTransient(server.status)}
                        >
                            Reboot
                        </Button>
                        <Button
                            color="warning"
                            onClick={() => onAction(server.server_id, 'shutdown')}
                            disabled={!['running'].includes(server.status) || isTransient(server.status)}
                        >
                            Shutdown
                        </Button>
                    </ButtonGroup>
                    <Button
                        variant="contained"
                        size="small"
                        color="error"
                        onClick={() => onDelete(server.server_id)}
                        disabled={!['stopped', 'failed'].includes(server.status) || isTransient(server.status)}
                    >
                        Delete
                    </Button>
                </Box>
            </TableCell>
        </TableRow>
    );
};

export default ServerTableRow;
