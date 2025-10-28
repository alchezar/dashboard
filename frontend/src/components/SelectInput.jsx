import React from 'react';
import {FormControl, InputLabel, Select, MenuItem} from '@mui/material';

const SelectInput = ({
                         name,
                         label,
                         value,
                         onChange,
                         options,
                         required = false
                     }) => {
    return (
        <FormControl fullWidth required={required}>
            <InputLabel id={`${name}-select-label`}>{label}</InputLabel>
            <Select
                variant="outlined"
                labelId={`${name}-select-label`}
                name={name}
                value={value}
                label={label}
                onChange={onChange}
            >
                {options.map((option) => (
                    <MenuItem key={option.value} value={option.value}>
                        {option.label || option.value}
                    </MenuItem>
                ))}
            </Select>
        </FormControl>
    );
};

export default SelectInput;
