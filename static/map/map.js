let baseUrl = "http://localhost:18032";
var map = L.map('map').setView([39.74739, -105], 13);

var tiles = L.tileLayer('https://api.mapbox.com/styles/v1/{id}/tiles/{z}/{x}/{y}?access_token=pk.eyJ1IjoibWFwYm94IiwiYSI6ImNpejY4NXVycTA2emYycXBndHRqcmZ3N3gifQ.rJcFIG214AriISLbB6B5aw', {
    maxZoom: 18,
    attribution: 'Map data &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors, ' +
        'Imagery © <a href="https://www.mapbox.com/">Mapbox</a>',
    id: 'mapbox/dark-v9',
    tileSize: 512,
    zoomOffset: -1
}).addTo(map);

L.Control.Command = L.Control.extend({
    options: {
        position: 'topleft',
    },

    onAdd: function(map) {
        var controlDiv = L.DomUtil.create('div', 'leaflet-control-command');
        L.DomEvent
            .addListener(controlDiv, 'click', L.DomEvent.stopPropagation)
            .addListener(controlDiv, 'click', L.DomEvent.preventDefault)
            .addListener(controlDiv, 'click', function() {
                PreviousDate();
            });

        var controlUI = L.DomUtil.create('div', 'leaflet-control-command-interior', controlDiv);
        controlUI.innerHTML = "Previous";
        return controlDiv;
    }
});
L.control.command1 = function(options) {
    return new L.Control.Command(options);
};
L.control.command1({}).addTo(map);

L.Control.Command = L.Control.extend({
    options: {
        position: 'topleft',
    },

    onAdd: function(map) {
        var controlDiv = L.DomUtil.create('div', 'leaflet-control-command');
        L.DomEvent
            .addListener(controlDiv, 'click', L.DomEvent.stopPropagation)
            .addListener(controlDiv, 'click', L.DomEvent.preventDefault)
            .addListener(controlDiv, 'click', function() {
                NextDate();
            });

        var controlUI = L.DomUtil.create('div', 'leaflet-control-command-interior', controlDiv);
        controlUI.innerHTML = "Next";
        return controlDiv;
    }
});
L.control.command2 = function(options) {
    return new L.Control.Command(options);
};
L.control.command2({}).addTo(map);

function NextDate() {
    date.setDate(date.getDate() + 1);
    updateData(date);
}

function PreviousDate() {
    date.setDate(date.getDate() - 1);
    updateData(date);
}


function onEachFeature(feature, layer) {
    var popupContent = "";

    if (feature.properties) {
        if (feature.properties.timestamp) {
            popupContent += "<p><b>" + feature.properties.timestamp + "</b></p>";
        }
        if (feature.properties.speed) {
            popupContent += "<p>" + 3.6 * feature.properties.speed + " km/h</p>";
        }
    }

    layer.bindPopup(popupContent);
}

let date = new Date();
let overlayLayer = null;

async function fetchDataJSON(date) {
    const response = await fetch(baseUrl + "/api/query?date=" + date.toISOString() + "&token=oPF2pkbvUBqmanC8ft9VbpwJwo9zK3HYkYhyAmps6rJzZHUmDmFGMQyysMnXymgK");
    const content = await response.json();
    return content;
}

function drawGeoJSON(json) {
    json.sort((obj) => Date.parse(obj.properties.timestamp));

    let times = json.map((obj) => Date.parse(obj.properties.timestamp));
    let time_max = Math.max(...times);
    let time_min = Math.min(...times);
    // json = json.filter(function(value, index, Arr) {
    //     return index % 5 == 0;
    // });

    let coords = json.map((d) => [d.geometry.coordinates[1], d.geometry.coordinates[0]]);
    let max_coords = coords.reduce(function(a, b) {
        return [Math.max(a[0], b[0]), Math.max(a[1], b[1])];
    }, [-Infinity, -Infinity]);
    let min_coords = coords.reduce(function(a, b) {
        return [Math.min(a[0], b[0]), Math.min(a[1], b[1])];
    }, [Infinity, Infinity]);
    map.setView([(max_coords[0] + min_coords[0]) / 2,
        (max_coords[1] + min_coords[1]) / 2
    ]);
    // let line = L.polyline(coords).addTo(map);

    overlayLayer = L.geoJSON(json, {

        style: function(feature) {
            return feature.properties && feature.properties.style;
        },

        onEachFeature: onEachFeature,

        pointToLayer: function(feature, latlng) {
            let dtime = Date.parse(feature.properties.timestamp);
            let percent = (dtime - time_min) / (time_max - time_min);
            let color = "rgb(" + (percent * 255) + ", " + (120 * percent) + ", " + ((1 - percent) * 255) + ")";
            return L.circleMarker(latlng, {
                radius: 3,
                fillColor: color,
                weight: 0,
                opacity: 0.5,
                fillOpacity: 0.5
            });
        }
    }).addTo(map);
    map.fitBounds(overlayLayer.getBounds());
}


function updateData(date) {
    if (overlayLayer) {
        map.removeLayer(overlayLayer);
    }
    fetchDataJSON(date)
        .then(drawGeoJSON)
        .catch(err => console.error(`Fetch problem: ${err.message}`));

}

updateData(date);
