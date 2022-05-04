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
                MapShowCommand();
            });

        var controlUI = L.DomUtil.create('div', 'leaflet-control-command-interior', controlDiv);
        controlUI.innerHTML = "Test";
        controlUI.title = 'Map Commands';
        return controlDiv;
    }
});

L.control.command = function(options) {
    return new L.Control.Command(options);
};
L.control.command({}).addTo(map);

function onEachFeature(feature, layer) {
    var popupContent = '<p>I started out as a GeoJSON ' +
        feature.geometry.type + ', but now I\'m a Leaflet vector!</p>';

    if (feature.properties && feature.properties.popupContent) {
        popupContent += feature.properties.popupContent;
    }

    layer.bindPopup(popupContent);
}
async function fetchDataJSON() {
    const response = await fetch(baseUrl + "/api/query?tz=2022-04-30T11:45:41Z&token=oPF2pkbvUBqmanC8ft9VbpwJwo9zK3HYkYhyAmps6rJzZHUmDmFGMQyysMnXymgK");
    const content = await response.json();
    return content;
}

fetchDataJSON()
    .then(json => {
        let coords = json.map((d) => d.geometry.coordinates);
        let avg = coords.reduce((a, b) => [(a[0] + b[0]), (a[1] + b[1])]);
        avg[0] /= coords.length;
        avg[1] /= coords.length;
        map.setView([coords[0][1], coords[0][0]], 13);
        var bicycleRentalLayer = L.geoJSON(json, {

            style: function(feature) {
                return feature.properties && feature.properties.style;
            },

            onEachFeature: onEachFeature,

            pointToLayer: function(feature, latlng) {
                return L.circleMarker(latlng, {
                    radius: 3,
                    fillColor: '#ff7800',
                    weight: 0,
                    opacity: 0.2,
                    fillOpacity: 0.2
                });
            }
        }).addTo(map);
    })
    .catch(err => console.error(`Fetch problem: ${err.message}`));