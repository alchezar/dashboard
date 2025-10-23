SELECT relid,
       fieldoptions
FROM tblcustomfields
WHERE type = 'product'
  AND (LOWER(fieldname) LIKE '%os%'
    OR LOWER(fieldname) LIKE '%temp%')
;