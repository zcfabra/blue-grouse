pub const GET_FOREIGN_KEYS: &str = "
SELECT
    tc.table_schema AS dependent_table_schema, 
    tc.constraint_name, 
    tc.table_name AS dependent_table_name, 
    kcu.column_name AS dependent_column_name, 
    ccu.table_schema AS foreign_table_schema,
    ccu.table_name AS foreign_table_name,
    ccu.column_name AS foreign_column_name 
FROM information_schema.table_constraints AS tc 
JOIN information_schema.key_column_usage AS kcu
    ON tc.constraint_name = kcu.constraint_name
    AND tc.table_schema = kcu.table_schema
JOIN information_schema.constraint_column_usage AS ccu
    ON ccu.constraint_name = tc.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY'
    AND ccu.table_schema=$1
    AND ccu.table_name=$2;
";

pub const GET_DEPENDENT_OBJECTS: &str = "
SELECT dependent_ns.nspname as dependent_schema
, dependent_view.relname as dependent_view 
, source_ns.nspname as source_schema
, source_table.relname as source_table
, ARRAY_AGG(pg_attribute.attname) as column_names
FROM pg_depend 
JOIN pg_rewrite ON pg_depend.objid = pg_rewrite.oid 
JOIN pg_class as dependent_view ON pg_rewrite.ev_class = dependent_view.oid 
JOIN pg_class as source_table ON pg_depend.refobjid = source_table.oid 
JOIN pg_attribute ON pg_depend.refobjid = pg_attribute.attrelid 
    AND pg_depend.refobjsubid = pg_attribute.attnum 
JOIN pg_namespace dependent_ns ON dependent_ns.oid = dependent_view.relnamespace
JOIN pg_namespace source_ns ON source_ns.oid = source_table.relnamespace
WHERE 
source_ns.nspname = $1
AND source_table.relname = $2
GROUP BY
	dependent_ns.nspname,
	dependent_view.relname,
	source_ns.nspname,
	source_table.relname
ORDER BY 1,2;
";
