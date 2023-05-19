drop table if exists bc_billa_category;
create table if not exists bc_billa_category (
    bc_id uuid default gen_random_uuid() primary key,
    bc_text character varying(256) not null
);
drop table if exists bcw_billa_crawl;
create table if not exists bcw_billa_crawl (
    bcw_id uuid default gen_random_uuid() primary key,
    bcw_created timestamp default current_timestamp
);
drop table if exists br_billa_raw;
create table if not exists br_billa_raw (
    br_id uuid default gen_random_uuid() primary key,
    br_raw text,
    br_created timestamp default current_timestamp,
    br_url character varying(256) not null,
    br_err text default null,
    br_bcw_crawl uuid not null,
);
ALTER TABLE br_billa_raw
ADD CONSTRAINT br_bcw_crawler_fk FOREIGN KEY (br_bcw_crawl) REFERENCES bcw_billa_crawl(bcw_id);
drop table if exists bpo_billa_product;
create table if not exists bpo_billa_product (
    bpo_id uuid default gen_random_uuid() primary key,
    bpo_created timestamp default current_timestamp,
    bpo_online_shop_url character varying(256) not null,
    bpo_billa_id character varying(16) not null,
    bpo_name character varying(256) not null,
    bpo_description text,
    bpo_brand character varying(256),
    bpo_badge character varying(256),
    bpo_unit character varying(256),
    bpo_price_factor float,
    bpo_grammage character varying(256),
    bpo_bc_category uuid not null
);
create unique index bpo_billa_product_bpo_billa_id_idx on bpo_billa_product(bpo_billa_id);
ALTER TABLE bpo_billa_product
ADD CONSTRAINT bpo_billa_category_fk FOREIGN KEY (bpo_bc_category) REFERENCES bc_billa_category(bc_id);
drop table if exists bp_billa_price;
create table if not exists bp_billa_price (
    bp_id uuid default gen_random_uuid() primary key,
    bp_created timestamp default current_timestamp,
    bp_normal float,
    bp_unit character varying(256),
    bp_bpo_product uuid not null,
    bp_br_raw uuid not null
);
ALTER TABLE bp_billa_price
ADD CONSTRAINT bp_billa_price_fk FOREIGN KEY (bp_bpo_product) REFERENCES bpo_billa_product(bpo_id);
ALTER TABLE bp_billa_price
ADD CONSTRAINT bp_billa_raw_fk FOREIGN KEY (bp_br_raw) REFERENCES br_billa_raw(br_id);