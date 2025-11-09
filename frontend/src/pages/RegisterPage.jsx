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

function RegisterPage() {
    const [formData, setFormData] = useState({
        first_name: '',
        last_name: '',
        email: '',
        password: '',
        address: '',
        city: '',
        state: '',
        post_code: '',
        country: '',
        phone_number: '',
    });
    const [error, setError] = useState('');
    const navigate = useNavigate();
    const {login} = useAuth();

    const handleChange = (event) => {
        setFormData({
            ...formData,
            [event.target.name]: event.target.value,
        });
    };

    const handleSubmit = async (event) => {
        event.preventDefault();
        setError('');
        try {
            const response = await api.post('/register', formData);

            await login(response.data.result.token);
            navigate('/');

        } catch (err) {
            console.error('Registration failed:', err);
            const errorMessage = err.response?.data?.message || 'Registration failed. Please check your data.';
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
                        Register
                    </Typography>
                    {error && <Alert severity="error" sx={{
                        width: '100%',
                        mt: 2
                    }}>{error}</Alert>}
                    <Box component="form" onSubmit={handleSubmit}
                         sx={{mt: 3, width: '100%'}}>
                        <Box sx={{mb: 2}}>
                            <TextField name="first_name" required fullWidth
                                       label="First Name"
                                       onChange={handleChange} autoFocus/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="last_name" required fullWidth
                                       label="Last Name"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="email" type="email" required
                                       fullWidth label="Email Address"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="password" type="password"
                                       required fullWidth label="Password"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="address" required fullWidth
                                       label="Address"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="city" required fullWidth
                                       label="City"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="state" required fullWidth
                                       label="State/Province"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="post_code" required fullWidth
                                       label="Postal Code"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="country" required fullWidth
                                       label="Country"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <TextField name="phone_number" required
                                       fullWidth label="Phone Number"
                                       onChange={handleChange}/>
                        </Box>
                        <Box sx={{mb: 2}}>
                            <Button
                                type="submit"
                                fullWidth
                                variant="contained"
                                color="success"
                                sx={{mb: 2}}
                            >
                                Register
                            </Button>
                        </Box>
                    </Box>
                </Box>
            </Paper>
        </Container>
    );
}

export default RegisterPage;
