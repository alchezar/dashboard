SELECT relid,
       fieldoptions
FROM tblcustomfields
WHERE type = 'product'
  AND fieldname = 'OS Template'
;