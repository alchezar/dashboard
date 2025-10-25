SELECT o.id,
       o.relid,
       o.configid,
       s.optionname
FROM tblhostingconfigoptions AS o
         JOIN tblproductconfigoptionssub AS s ON o.optionid = s.id
;