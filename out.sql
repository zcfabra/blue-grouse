-- DELETE DEPDENDENTS

-- VIEW location.vw_room
DROP VIEW location.vw_room;

-- FK fk_room_settings_room_pin
ALTER TABLE settings.room_config
DROP CONSTRAINT fk_room_settings_room_pin;

-- VIEW vw_room
CREATE VIEW location.vw_room AS
 SELECT r.pin,
    r.name,
    r.foo,
    r.bar,
    rc.config_option
   FROM (location.room r
     JOIN settings.room_config rc ON ((r.pin = rc.room_pin)));








-- ADD BACK DEPENDENTS

-- FK fk_room_settings_room_pin
ALTER TABLE ONLY settings.room_config
    ADD CONSTRAINT fk_room_settings_room_pin FOREIGN KEY (room_pin) REFERENCES location.room(pin);