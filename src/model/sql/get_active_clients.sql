SELECT c.id,
       c.firstname,
       c.lastname,
       c.email,
       c.address1,
       c.city,
       c.state,
       c.postcode,
       c.country,
       c.phonenumber,
       c.password,
       c.created_at,
       c.updated_at
FROM tblclients AS c
         JOIN tblhosting AS h ON c.id = h.userid
WHERE h.domainstatus = 'Active'
;
