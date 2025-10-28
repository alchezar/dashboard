import React, {useState, useEffect} from 'react';
import {
    Modal,
    Box,
    Typography,
    TextField,
    Button,
    Paper,
    Alert,
    CircularProgress
} from '@mui/material';
import api from '../services/api';
import SelectInput from './SelectInput';

const style = {
    position: 'absolute',
    top: '50%',
    left: '50%',
    transform: 'translate(-50%, -50%)',
    width: 600,
    bgcolor: 'background.paper',
    boxShadow: 24,
    p: 4,
};

function CreateServerModal({open, handleClose}) {
    const [formData, setFormData] = useState({
        product_id: '',
        host_name: '',
        cpu_cores: '',
        ram_gb: '',
        os: '',
        datacenter: '',
    });
    const [error, setError] = useState('');
    const [loading, setLoading] = useState(false);
    const [products, setProducts] = useState([]);
    const [cpuOptions, setCpuOptions] = useState([]);
    const [ramOptions, setRamOptions] = useState([]);
    const [osOptions, setOsOptions] = useState([]);
    const [datacenterOptions, setDatacenterOptions] = useState([]);

    useEffect(() => {
        if (open) {
            (async () => {
                try {
                    const [productsRes, cpusRes, ramsRes, osRes, dcRes] = await Promise.all([
                        api.get('api/products'),
                        api.get('api/config/cpu'),
                        api.get('api/config/ram'),
                        api.get('api/custom/os'),
                        api.get('api/custom/datacenter'),
                    ]);
                    setProducts(productsRes.data.result.map(p => ({
                        value: p.id,
                        label: p.name
                    })) || []);
                    setCpuOptions(cpusRes.data.result.map(o => ({value: o.value})) || []);
                    setRamOptions(ramsRes.data.result.map(o => ({value: o.value})) || []);
                    setOsOptions(osRes.data.result.map(o => ({value: o.value})) || []);
                    setDatacenterOptions(dcRes.data.result.map(o => ({value: o.value})) || []);
                } catch (err) {
                    setError('Failed to load creation options.');
                    console.error(err);
                }
            })();
        }
    }, [open]);

    const handleChange = (event) => {
        setFormData({
            ...formData,
            [event.target.name]: event.target.value,
        });
    };

    const handleSubmit = async (event) => {
        event.preventDefault();
        setError('');
        setLoading(true);

        const payload = {
            ...formData,
            cpu_cores: formData.cpu_cores ? parseInt(formData.cpu_cores, 10) : undefined,
            ram_gb: formData.ram_gb ? parseInt(formData.ram_gb, 10) : undefined,
        };

        try {
            await api.post('/servers', payload);
            handleClose(true);
        } catch (err) {
            console.error('Server creation failed:', err);
            const errorMessage = err.response?.data?.message || 'Server creation failed. Please check your data.';
            setError(errorMessage);
        } finally {
            setLoading(false);
        }
    };

    return (
        <Modal
            open={open}
            onClose={() => handleClose(false)}
            aria-labelledby="create-server-modal-title"
        >
            <Box sx={style} component={Paper}>
                <Typography id="create-server-modal-title" variant="h6"
                            component="h2">
                    Create New Server
                </Typography>
                {error && <Alert severity="error"
                                 sx={{width: '100%', mt: 2}}>{error}</Alert>}
                <Box component="form" onSubmit={handleSubmit} sx={{
                    mt: 2,
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 2
                }}>
                    <SelectInput name="product_id" label="Product"
                                 value={formData.product_id}
                                 onChange={handleChange} options={products}
                                 required/>
                    <TextField name="host_name" required fullWidth
                               label="Host Name" onChange={handleChange}
                               value={formData.host_name}/>
                    <SelectInput name="cpu_cores" label="CPU Cores"
                                 value={formData.cpu_cores}
                                 onChange={handleChange} options={cpuOptions}
                                 required/>
                    <SelectInput name="ram_gb" label="RAM (GB)"
                                 value={formData.ram_gb} onChange={handleChange}
                                 options={ramOptions} required/>
                    <SelectInput name="os" label="Operating System"
                                 value={formData.os} onChange={handleChange}
                                 options={osOptions} required/>
                    <SelectInput name="datacenter" label="Datacenter"
                                 value={formData.datacenter}
                                 onChange={handleChange}
                                 options={datacenterOptions} required/>
                    <Box sx={{
                        mt: 3,
                        display: 'flex',
                        justifyContent: 'flex-end'
                    }}>
                        <Button onClick={() => handleClose(false)}
                                sx={{mr: 1}}>Cancel</Button>
                        <Button
                            type="submit"
                            variant="contained"
                            color="primary"
                            disabled={loading}
                        >
                            {loading ? <CircularProgress size={24}/> : 'Create'}
                        </Button>
                    </Box>
                </Box>
            </Box>
        </Modal>
    );
}

export default CreateServerModal;
