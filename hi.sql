ALTER TABLE ONLY settings.room_config
    ADD CONSTRAINT thing PRIMARY KEY (room_pin) REFERENCES location.room(pin);

ALTER TABLE ONLY settings.room_config
    ADD CONSTRAINT thing FOREIGN KEY (thing) REFERENCES location.room(pin);

ALTER TABLE ONLY settings.room_config
    ADD CONSTRAINT fk_room_settings_room_pin FOREIGN KEY (room_pin) REFERENCES location.room(pin);