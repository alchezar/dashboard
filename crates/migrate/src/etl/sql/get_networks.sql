SELECT DISTINCT p.id,
                p.title,
                p.gateway,
                a.mask
FROM mod_pvewhmcs_ip_pools p
         JOIN mod_pvewhmcs_ip_addresses a ON p.id = a.pool_id
;