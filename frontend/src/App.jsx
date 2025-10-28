import {Routes, Route, Link as RouterLink} from 'react-router-dom';
import AppBar from '@mui/material/AppBar';
import Box from '@mui/material/Box';
import Toolbar from '@mui/material/Toolbar';
import Typography from '@mui/material/Typography';
import Container from '@mui/material/Container';

import LoginPage from './pages/LoginPage';
import RegisterPage from './pages/RegisterPage';

import DashboardPage from './pages/DashboardPage';
import ProtectedRoute from './components/ProtectedRoute';
import AuthButtons from './components/AuthButtons';

function App() {
    return (
        <>
            <Box sx={{flexGrow: 1}}>
                <AppBar position="static">
                    <Toolbar>
                        <Typography variant="h6" component="div"
                                    sx={{flexGrow: 1}}>
                            <RouterLink to="/" style={{
                                textDecoration: 'none',
                                color: 'inherit'
                            }}>
                                Dashboard
                            </RouterLink>
                        </Typography>
                        <AuthButtons/>
                    </Toolbar>
                </AppBar>
            </Box>
            <Container sx={{mt: 4}}>
                <Routes>
                    <Route
                        path="/"
                        element={
                            <ProtectedRoute>
                                <DashboardPage/>
                            </ProtectedRoute>
                        }
                    />
                    <Route path="/login" element={<LoginPage/>}/>
                    <Route path="/register" element={<RegisterPage/>}/>
                </Routes>
            </Container>
        </>
    );
}

export default App;
