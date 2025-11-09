import {createTheme} from '@mui/material/styles';

const theme = createTheme({
    palette: {
        primary: {
            main: '#0d5aa7',
        },
        success: {
            main: '#28a745',
        },
        warning: {
            main: '#ffc107',
        },
        error: {
            main: '#dc3545',
        },
        background: {
            default: '#fafafa',
        },
    },
    typography: {
        fontFamily: [
            'sans-serif',
        ].join(','),
        h5: {
            fontWeight: 700
        }
    },
    components: {
        MuiAppBar: {
            styleOverrides: {
                root: {
                    backgroundColor: '#212529',
                    color: '#fff',
                },
            },
        },
        MuiButton: {
            styleOverrides: {
                root: {
                    textTransform: 'none',
                    fontWeight: 'bold',
                },
            },
        },
    },
});

export default theme;
