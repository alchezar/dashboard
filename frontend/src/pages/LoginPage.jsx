import React, {useState} from 'react';
import {
    Container,
    Box,
    Typography,
    TextField,
    Button,
    Paper,
    Alert,
} from '@mui/material';
import {useNavigate} from 'react-router-dom';
import api from '../services/api';
import {useAuth} from '../context/AuthContext';

function LoginPage() {
    const [email, setEmail] = useState('');
    const [password, setPassword] = useState('');
    const [error, setError] = useState('');
    const navigate = useNavigate();
    const {login} = useAuth();

    const handleSubmit = async (event) => {
        event.preventDefault();
        setError('');
        try {
            const response = await api.post('/login', {
                email,
                password,
            });

            await login(response.data.result.token);
            navigate('/');

        } catch (err) {
            console.error('Login failed:', err);
            const errorMessage = err.response?.data?.message || 'Login failed. Please check your credentials.';
            setError(errorMessage);
        }
    };

    return (
        <Container maxWidth="sm">
            <Paper elevation={3} sx={{mt: 8, p: 4}}>
                <Box
                    sx={{
                        display: 'flex',
                        flexDirection: 'column',
                        alignItems: 'center',
                    }}
                >
                    <Typography component="h1" variant="h5">
                        Login
                    </Typography>
                    {error && <Alert severity="error" sx={{
                        width: '100%',
                        mt: 2
                    }}>{error}</Alert>}
                    <Box component="form" onSubmit={handleSubmit} sx={{mt: 1}}>
                        <TextField
                            margin="normal"
                            required
                            fullWidth
                            id="email"
                            label="Email Address"
                            name="email"
                            autoComplete="email"
                            autoFocus
                            value={email}
                            onChange={(e) => setEmail(e.target.value)}
                        />
                        <TextField
                            margin="normal"
                            required
                            fullWidth
                            name="password"
                            label="Password"
                            type="password"
                            id="password"
                            autoComplete="current-password"
                            value={password}
                            onChange={(e) => setPassword(e.target.value)}
                        />
                        <Button
                            type="submit"
                            fullWidth
                            variant="contained"
                            color="success"
                            sx={{mt: 3, mb: 2}}
                        >
                            Login
                        </Button>
                    </Box>
                </Box>
            </Paper>
        </Container>
    );
}

export default LoginPage;
