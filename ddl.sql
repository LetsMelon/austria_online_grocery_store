drop table if exists bc_billa_category;
create table if not exists bc_billa_category (
    bc_id uuid default gen_random_uuid() primary key,
    bc_text character varying(256) not null
);
cs_crawl_session drop table if exists bcw_billa_crawl;
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
drop table if exists sc_spar_category;
create table if not exists sc_spar_category (
    sc_id uuid default gen_random_uuid() primary key,
    sc_text character varying(256) not null
);
drop table if exists sr_spar_raw;
create table if not exists sr_spar_raw (
    sr_id uuid default gen_random_uuid() primary key,
    sr_raw text,
    sr_created timestamp default current_timestamp,
    sr_url character varying(256) not null,
    sr_err text default null,
    sr_cs_crawl_session uuid not null
);
ALTER TABLE sr_spar_raw
ADD CONSTRAINT sr_spar_raw_crawler_fk FOREIGN KEY (sr_cs_crawl_session) REFERENCES bcw_billa_crawl(bcw_id);
drop table if exists sp_spar_product;
create table if not exists sp_spar_product (
    sp_id uuid default gen_random_uuid() primary key,
    sp_created timestamp default current_timestamp,
    sp_spar_id character varying(13) not null,
    sp_description text,
    sp_online_shop_url character varying(256) not null,
    sp_name character varying(256) not null,
    sp_brand character varying(256),
    sp_sc_category uuid not null
);
ALTER TABLE sp_spar_product
ADD CONSTRAINT sp_spar_product_category_fk FOREIGN KEY (sp_sc_category) REFERENCES sc_spar_category(sc_id);
drop table if exists spr_spar_price;
create table if not exists spr_spar_price (
    spr_id uuid default gen_random_uuid() primary key,
    spr_p_created timestamp default current_timestamp,
    spr_price float,
    spr_sales_unit character varying(256),
    spr_price_unit character varying(256),
    spr_sp_product uuid not null,
    spr_sr_raw uuid not null
);
ALTER TABLE spr_spar_price
ADD CONSTRAINT spr_spar_price_product_fk FOREIGN KEY (spr_sp_product) REFERENCES sp_spar_product(sp_id);
ALTER TABLE spr_spar_price
ADD CONSTRAINT spr_spar_price_raw_fk FOREIGN KEY (spr_sr_raw) REFERENCES sr_spar_raw(sr_id);