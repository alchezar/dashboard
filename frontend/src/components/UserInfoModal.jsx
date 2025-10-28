import React from 'react';
import {
    Modal,
    Box,
    Typography,
    Paper,
    Button,
} from '@mui/material';

const style = {
    position: 'absolute',
    top: '50%',
    left: '50%',
    transform: 'translate(-50%, -50%)',
    width: 400,
    bgcolor: 'background.paper',
    boxShadow: 24,
    p: 4,
};

const UserInfoRow = ({label, value}) => (
    <Box sx={{display: 'flex', justifyContent: 'space-between'}}>
        <Typography variant="subtitle1" color="text.secondary">
            {label}:
        </Typography>
        <Typography>{value || 'N/A'}</Typography>
    </Box>
);

function UserInfoModal({open, handleClose, user}) {
    if (!user) {
        return null;
    }

    const userInfoFields = [
        {label: 'First Name', key: 'first_name'},
        {label: 'Last Name', key: 'last_name'},
        {label: 'Email', key: 'email'},
        {label: 'Address', key: 'address'},
        {label: 'City', key: 'city'},
        {label: 'State', key: 'state'},
        {label: 'Post Code', key: 'post_code'},
        {label: 'Country', key: 'country'},
        {label: 'Phone Number', key: 'phone_number'},
    ];

    return (
        <Modal
            open={open}
            onClose={handleClose}
            aria-labelledby="user-info-modal-title"
        >
            <Box sx={style} component={Paper}>
                <Typography id="user-info-modal-title" variant="h6"
                            component="h2">
                    User Information
                </Typography>
                <Box sx={{
                    mt: 2,
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 1
                }}>
                    {userInfoFields.map((field) => (
                        user[field.key] ?
                            <UserInfoRow key={field.key} label={field.label}
                                         value={user[field.key]}/> : null
                    ))}
                </Box>
                <Box sx={{mt: 3, display: 'flex', justifyContent: 'flex-end'}}>
                    <Button onClick={handleClose}>Close</Button>
                </Box>
            </Box>
        </Modal>
    );
}

export default UserInfoModal;
