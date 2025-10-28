import React, {useState} from 'react';
import {Link as RouterLink} from 'react-router-dom';
import Button from '@mui/material/Button';
import {useAuth} from '../context/AuthContext';
import UserInfoModal from './UserInfoModal';

function AuthButtons() {
    const {isAuthenticated, user, logout} = useAuth();
    const [modalOpen, setModalOpen] = useState(false);

    const handleOpenModal = () => setModalOpen(true);
    const handleCloseModal = () => setModalOpen(false);

    if (isAuthenticated) {
        return (
            <>
                <Button color="inherit" onClick={handleOpenModal}>
                    {user?.first_name} {user?.last_name || 'User'}
                </Button>
                <Button color="inherit" onClick={logout}>Logout</Button>
                <UserInfoModal open={modalOpen} handleClose={handleCloseModal}
                               user={user}/>
            </>
        );
    }

    return (
        <>
            <Button color="inherit" component={RouterLink}
                    to="/login">Login</Button>
            <Button color="inherit" component={RouterLink}
                    to="/register">Register</Button>
        </>
    );
}

export default AuthButtons;
