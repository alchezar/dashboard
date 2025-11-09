SELECT id,
       domainstatus,
       userid,
       packageid
FROM tblhosting
WHERE domainstatus = 'Active'
;