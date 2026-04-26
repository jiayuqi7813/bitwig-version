package com.bitwig.versions;

import com.bitwig.extension.controller.ControllerExtension;
import com.bitwig.extension.controller.ControllerExtensionDefinition;

import java.util.UUID;

public class BitwigVersionsExtensionDefinition extends ControllerExtensionDefinition {
    private static final UUID DRIVER_ID = UUID.fromString("5b9735e7-83ff-4787-a9c7-34c8c01a6dd1");

    @Override
    public String getName() {
        return "Bitwig Versions";
    }

    @Override
    public String getAuthor() {
        return "Bitwig Versions";
    }

    @Override
    public String getVersion() {
        return "0.1.0";
    }

    @Override
    public UUID getId() {
        return DRIVER_ID;
    }

    @Override
    public String getHardwareVendor() {
        return "Bitwig Versions";
    }

    @Override
    public String getHardwareModel() {
        return "Virtual Controller";
    }

    @Override
    public int getRequiredAPIVersion() {
        return 18;
    }

    @Override
    public ControllerExtension createInstance(final com.bitwig.extension.controller.api.ControllerHost host) {
        return new BitwigVersionsExtension(this, host);
    }
}
