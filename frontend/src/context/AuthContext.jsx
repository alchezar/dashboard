import React, {
    createContext,
    useState,
    useEffect,
    useCallback,
    useContext
} from 'react';
import api from '../services/api';
import {useNavigate} from 'react-router-dom';

const AuthContext = createContext(null);

export const AuthProvider = ({children}) => {
    const [isAuthenticated, setIsAuthenticated] = useState(false);
    const [user, setUser] = useState(null);
    const [loading, setLoading] = useState(true);
    const navigate = useNavigate();

    const fetchUser = useCallback(async () => {
        try {
            const response = await api.get('/user/me');
            setUser(response.data.result);
            setIsAuthenticated(true);
        } catch (err) {
            console.error('Failed to fetch user data:', err);
            localStorage.removeItem('authToken');
            setIsAuthenticated(false);
            setUser(null);
            navigate('/login');
        } finally {
            setLoading(false);
        }
    }, [navigate]);

    useEffect(() => {
        const token = localStorage.getItem('authToken');
        if (token) {
            void fetchUser();
        } else {
            setLoading(false);
        }
    }, [fetchUser]);

    const login = useCallback(async (token) => {
        localStorage.setItem('authToken', token);
        await fetchUser();
    }, [fetchUser]);

    const logout = useCallback(() => {
        localStorage.removeItem('authToken');
        setIsAuthenticated(false);
        setUser(null);
        navigate('/login');
    }, [navigate]);

    const value = {
        isAuthenticated,
        user,
        loading,
        login,
        logout,
    };

    return <AuthContext.Provider
        value={value}>{children}</AuthContext.Provider>;
};

export const useAuth = () => {
    return useContext(AuthContext);
};
