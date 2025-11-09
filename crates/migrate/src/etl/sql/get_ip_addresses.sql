SELECT adr.id,
       adr.pool_id,
       adr.ipaddress,
       vms.id AS server_id
FROM mod_pvewhmcs_ip_addresses adr
         LEFT JOIN mod_pvewhmcs_vms vms ON adr.ipaddress = vms.ipaddress
;