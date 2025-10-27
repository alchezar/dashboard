SELECT vms.id,
       vms.vmid,
       srv.name         AS node,
       hst.domain       AS hostname,
       hst.domainstatus AS status
FROM tblhosting hst
         JOIN mod_pvewhmcs_vms vms ON hst.userid = vms.user_id
         LEFT JOIN tblservers srv ON vms.node_id = srv.id
WHERE hst.domainstatus = 'Active'
;
